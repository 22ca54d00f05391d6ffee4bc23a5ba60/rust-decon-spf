//! This module allows you to deconstruct an existing SPF DNS record into its
//! constituent parts.  
//! It is not intended to validate the spf record.

mod errors;
mod tests;
mod validate;

use crate::helpers;
use crate::mechanism::Kind;
pub use crate::mechanism::{Mechanism, ParsedMechanism};
pub use crate::spf::errors::SpfError;
use ipnetwork::IpNetwork;
// Make this public in the future
use crate::spf::validate::{SpfRfcStandard, SpfValidationResult};
use std::{convert::TryFrom, str::FromStr};

/// This is the maximnum number of characters that an Spf Record can store.
const MAX_SPF_STRING_LENGTH: usize = 255;

/// The definition of the Spf struct which contains all information related a single
/// SPF record.
#[derive(Debug)]
pub struct Spf {
    source: String,
    version: String,
    from_src: bool,
    redirect: Option<Mechanism<String>>,
    is_redirected: bool,
    a: Option<Vec<Mechanism<String>>>,
    mx: Option<Vec<Mechanism<String>>>,
    include: Option<Vec<Mechanism<String>>>,
    ip4: Option<Vec<Mechanism<IpNetwork>>>,
    ip6: Option<Vec<Mechanism<IpNetwork>>>,
    ptr: Option<Mechanism<String>>,
    exists: Option<Vec<Mechanism<String>>>,
    all: Option<Mechanism<String>>,
    was_parsed: bool,
    was_validated: bool,
    is_valid: bool,
    warnings: Option<Vec<String>>,
}

impl std::fmt::Display for Spf {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.build_spf_string())
    }
}

impl Default for Spf {
    fn default() -> Self {
        Self {
            source: String::new(),
            version: String::new(),
            from_src: false,
            redirect: None,
            is_redirected: false,
            a: None,
            mx: None,
            include: None,
            ip4: None,
            ip6: None,
            ptr: None,
            exists: None,
            all: None,
            was_parsed: false,
            was_validated: false,
            is_valid: false,
            warnings: None,
        }
    }
}

/// Creates an `Spf Stuct` by parsing a spf string.
///
/// # examples
///
///```rust
/// use decon_spf::Spf;
/// use decon_spf::SpfError;
/// // Successful
/// let input = "v=spf1 a mx -all";
/// let spf: Spf = input.to_string().parse().unwrap();
/// assert_eq!(spf.to_string(), input);
///
/// // Additional Space between `A` and `MX`
/// let bad_input = "v=spf1 a   mx -all";
/// let err: SpfError = bad_input.to_string().parse::<Spf>().unwrap_err();
/// assert_eq!(err.to_string(), SpfError::WhiteSpaceSyntaxError.to_string());
/// //  err.to_string() -> "Spf contains two or more consecutive whitespace characters.");
///```
///
impl FromStr for Spf {
    type Err = SpfError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let source = String::from(s);
        if !source.starts_with("v=spf1") && !source.starts_with("spf2.0") {
            return Err(SpfError::InvalidSource);
        };
        if source.len() > MAX_SPF_STRING_LENGTH {
            return Err(SpfError::SourceLengthExceeded);
        };
        if helpers::spf_check_whitespace(source.as_str()) {
            return Err(SpfError::WhiteSpaceSyntaxError);
        };
        // Basic Checks are ok.
        let mut spf = Spf::new();
        // Setup Vecs
        let records = source.split_whitespace();
        let mut vec_of_includes: Vec<Mechanism<String>> = Vec::new();
        let mut vec_of_ip4: Vec<Mechanism<IpNetwork>> = Vec::new();
        let mut vec_of_ip6: Vec<Mechanism<IpNetwork>> = Vec::new();
        let mut vec_of_a: Vec<Mechanism<String>> = Vec::new();
        let mut vec_of_mx: Vec<Mechanism<String>> = Vec::new();
        let mut vec_of_exists: Vec<Mechanism<String>> = Vec::new();
        #[cfg(feature = "warn-dns")]
        let mut vec_of_warnings: Vec<String> = Vec::new();
        for record in records {
            // Consider ensuring we do this once at least and then skip
            if record.contains("v=spf1") || record.starts_with("spf2.0") {
                spf.version = record.to_string();
            } else if record.contains("redirect=") {
                // Match a redirect
                if let Ok(redirect) = Mechanism::<String>::from_str(record) {
                    #[cfg(feature = "warn-dns")]
                    {
                        if !helpers::dns_is_valid(&redirect.raw()) {
                            vec_of_warnings.push(redirect.raw());
                        }
                    }
                    spf.redirect = Some(redirect);
                    spf.is_redirected = true;
                }
            } else if record.contains("include:") {
                if let Ok(include) = Mechanism::<String>::from_str(record) {
                    #[cfg(feature = "warn-dns")]
                    {
                        if !helpers::dns_is_valid(&include.raw()) {
                            vec_of_warnings.push(include.raw());
                        }
                    }
                    vec_of_includes.push(include);
                }
            } else if record.contains("exists:") {
                if let Ok(exists) = Mechanism::<String>::from_str(record) {
                    #[cfg(feature = "warn-dns")]
                    {
                        if !helpers::dns_is_valid(&exists.raw()) {
                            vec_of_warnings.push(exists.raw());
                        }
                    }
                    vec_of_exists.push(exists);
                }
            } else if record.contains("ip4:") {
                // Match an ip4
                let qualifier_and_modified_str = helpers::return_and_remove_qualifier(record, 'i');
                if let Some(raw_ip4) = qualifier_and_modified_str.1.strip_prefix("ip4:") {
                    let valid_ip4 = raw_ip4.parse();
                    match valid_ip4 {
                        Ok(ip4) => {
                            let network = Mechanism::new_ip4(qualifier_and_modified_str.0, ip4);
                            vec_of_ip4.push(network);
                        }
                        Err(ip4) => return Err(SpfError::InvalidIPAddr(ip4)),
                    }
                }
            } else if record.contains("ip6:") {
                // Match an ip6
                let qualifier_and_modified_str = helpers::return_and_remove_qualifier(record, 'i');
                if let Some(raw_ip6) = qualifier_and_modified_str.1.strip_prefix("ip6:") {
                    let valid_ip6 = raw_ip6.parse();
                    match valid_ip6 {
                        Ok(ip6) => {
                            let network = Mechanism::new_ip6(qualifier_and_modified_str.0, ip6);
                            vec_of_ip6.push(network);
                        }
                        Err(ip6) => return Err(SpfError::InvalidIPAddr(ip6)),
                    }
                }
            } else if record.ends_with("all") {
                // deal with all if present
                spf.all = Some(Mechanism::<String>::from_str(record).unwrap());
            // Handle A, MX and PTR types.
            } else if let Some(a_mechanism) = helpers::capture_matches(record, Kind::A) {
                #[cfg(feature = "warn-dns")]
                {
                    if !a_mechanism.raw().starts_with('/')
                        && !helpers::dns_is_valid(helpers::get_domain_before_slash(
                            &a_mechanism.raw(),
                        ))
                    {
                        vec_of_warnings.push(a_mechanism.raw());
                    }
                }
                vec_of_a.push(a_mechanism);
            } else if let Some(mx_mechanism) = helpers::capture_matches(record, Kind::MX) {
                #[cfg(feature = "warn-dns")]
                {
                    if !mx_mechanism.raw().starts_with('/')
                        && !helpers::dns_is_valid(helpers::get_domain_before_slash(
                            &mx_mechanism.raw(),
                        ))
                    {
                        vec_of_warnings.push(mx_mechanism.raw());
                    }
                }
                vec_of_mx.push(mx_mechanism);
            } else if let Some(ptr_mechanism) = helpers::capture_matches(record, Kind::Ptr) {
                #[cfg(feature = "warn-dns")]
                {
                    if !helpers::dns_is_valid(&ptr_mechanism.raw()) {
                        vec_of_warnings.push(ptr_mechanism.raw());
                    }
                }
                spf.ptr = Some(ptr_mechanism);
            }
        }
        // Move vec_of_* int the SPF struct
        if !vec_of_includes.is_empty() {
            spf.include = Some(vec_of_includes);
        };
        if !vec_of_ip4.is_empty() {
            spf.ip4 = Some(vec_of_ip4);
        };
        if !vec_of_ip6.is_empty() {
            spf.ip6 = Some(vec_of_ip6);
        };
        if !vec_of_a.is_empty() {
            spf.a = Some(vec_of_a);
        }
        if !vec_of_mx.is_empty() {
            spf.mx = Some(vec_of_mx);
        }
        if !vec_of_exists.is_empty() {
            spf.exists = Some(vec_of_exists);
        }
        #[cfg(feature = "warn-dns")]
        {
            if !vec_of_warnings.is_empty() {
                spf.warnings = Some(vec_of_warnings);
            }
        }

        spf.was_parsed = true;
        spf.is_valid = true;
        spf.source = source;
        Ok(spf)
    }
}

impl TryFrom<&str> for Spf {
    type Error = SpfError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Spf::from_str(s)
    }
}
impl Spf {
    /// Create a new empty Spf struct.
    pub fn new() -> Self {
        Spf::default()
    }
    /// Check that the source string was parsed and was valid.
    //pub fn source_is_vaid(&self) -> bool {
    //  // Should I check was validated?
    //    self.source_is_valid
    //}
    /// Check that data stored in the Spf Struct is considered a valid Spf Record.
    pub fn is_valid(&self) -> bool {
        if self.was_parsed || self.was_validated {
            return self.is_valid;
        };
        false
    }
    /// Check if there were any warnings when parsing the Spf String.
    /// This can only be changed to `true` when `warn-dns` feature has been eabled. Other wise it
    /// will always be `false`
    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_none()
    }
    /// Set version to `v=spf1`
    pub fn set_v1(&mut self) {
        self.version = String::from("v=spf1");
    }
    /// Set version to `spf2.0/pra`
    pub fn set_v2_pra(&mut self) {
        self.version = String::from("spf2.0/pra");
    }
    /// Set version to `spf2.0/mfrom`
    pub fn set_v2_mfrom(&mut self) {
        self.version = String::from("spf2.0/mfrom");
    }
    /// Set version to `spf2.0/pra,mfrom`
    pub fn set_v2_pra_mfrom(&mut self) {
        self.version = String::from("spf2.0/pra,mfrom");
    }
    /// Set version to `spf2.0/mfrom,pra`
    pub fn set_v2_mfrom_pra(&mut self) {
        self.version = String::from("spf2.0/mfrom,pra");
    }
    /// Check that version is v1
    pub fn is_v1(&self) -> bool {
        self.version.contains("v=spf1")
    }
    /// Check that version is v2
    pub fn is_v2(&self) -> bool {
        self.version.starts_with("spf2.0")
    }
    /// Return a reference to version
    pub fn version(&self) -> &String {
        &self.version
    }
    /// Append a Redirect Mechanism to the Spf Struct.
    fn append_mechanism_of_redirect(&mut self, mechanism: Mechanism<String>) {
        self.redirect = Some(mechanism);
        self.is_redirected = true;
        if self.all.is_some() {
            self.all = None;
        }
    }
    /// Clear the passed Kind which has been passed.
    /// Sets the passed mechanism to `None`
    ///
    /// # Note:
    /// This method clears all assocated Mechanism for the [`Kind`](Kind) provided.
    ///
    /// # Example:
    /// ```
    /// use decon_spf::mechanism::{Qualifier, Kind, Mechanism};
    /// use decon_spf::Spf;
    /// let mut new_spf_record = Spf::new();
    /// new_spf_record.set_v1();
    /// new_spf_record.append_mechanism(Mechanism::new_all(Qualifier::Pass));
    /// new_spf_record.append_mechanism(Mechanism::new_a_without_mechanism(Qualifier::Pass));
    /// new_spf_record.append_ip_mechanism(Mechanism::new_ip(Qualifier::Pass,
    ///                                                      "203.32.160.0/23".parse().unwrap()));
    /// assert_eq!(new_spf_record.to_string(), "v=spf1 a ip4:203.32.160.0/23 all".to_string());
    /// // Remove ip4 Mechanism
    /// new_spf_record.clear_mechanism(Kind::IpV4);
    /// assert_eq!(new_spf_record.to_string(), "v=spf1 a all".to_string());
    ///```
    pub fn clear_mechanism(&mut self, kind: Kind) {
        match kind {
            Kind::Redirect => {
                self.redirect = None;
                self.is_redirected = false;
            }
            Kind::A => self.a = None,
            Kind::MX => self.mx = None,
            Kind::Include => self.include = None,
            Kind::IpV4 => self.ip4 = None,
            Kind::IpV6 => self.ip6 = None,
            Kind::Exists => self.exists = None,
            Kind::Ptr => self.ptr = None,
            Kind::All => self.all = None,
        }
    }

    fn append_mechanism_of_a(&mut self, mechanism: Mechanism<String>) {
        if let Some(a) = &mut self.a {
            a.push(mechanism);
        } else {
            self.a = Some(vec![mechanism]);
        }
    }
    fn append_mechanism_of_mx(&mut self, mechanism: Mechanism<String>) {
        if let Some(mx) = &mut self.mx {
            mx.push(mechanism);
        } else {
            self.mx = Some(vec![mechanism]);
        }
    }
    fn append_mechanism_of_include(&mut self, mechanism: Mechanism<String>) {
        if let Some(include) = &mut self.include {
            include.push(mechanism);
        } else {
            self.include = Some(vec![mechanism]);
        }
    }
    fn append_mechanism_of_ip4(&mut self, mechanism: Mechanism<IpNetwork>) {
        if let Some(ip4) = &mut self.ip4 {
            ip4.push(mechanism);
        } else {
            self.ip4 = Some(vec![mechanism]);
        }
    }
    fn append_mechanism_of_ip6(&mut self, mechanism: Mechanism<IpNetwork>) {
        if let Some(ip6) = &mut self.ip6 {
            ip6.push(mechanism);
        } else {
            self.ip6 = Some(vec![mechanism]);
        }
    }
    fn append_mechanism_of_exists(&mut self, mechanism: Mechanism<String>) {
        if let Some(exists) = &mut self.exists {
            exists.push(mechanism);
        } else {
            self.exists = Some(vec![mechanism]);
        }
    }
    fn append_mechanism_of_ptr(&mut self, mechanism: Mechanism<String>) {
        self.ptr = Some(mechanism);
    }
    fn append_mechanism_of_all(&mut self, mechanism: Mechanism<String>) {
        if self.redirect.is_none() {
            self.all = Some(mechanism);
        }
    }
    /// Appends the passed `Mechanism<String>` to the SPF struct.
    /// This only works for Mechanism which are *NOT* `ip4:` or `ip6:`
    ///
    /// # Example:
    /// ```
    /// use decon_spf::mechanism::{Qualifier, Mechanism};
    /// use decon_spf::Spf;
    /// let mut new_spf_record = Spf::new();
    /// new_spf_record.set_v1();
    /// new_spf_record.append_mechanism(Mechanism::new_redirect(Qualifier::Pass,
    ///                                 "_spf.example.com".to_string()));
    /// new_spf_record.append_mechanism(Mechanism::new_all(Qualifier::Pass));
    /// assert_eq!(new_spf_record.to_string(), "v=spf1 redirect=_spf.example.com".to_string());
    /// ```
    ///
    /// # Note:
    /// If The Spf is already set as `Redirect` trying to append an `All`
    /// Mechanism will have no affect.
    // Consider make this a Result
    pub fn append_mechanism(&mut self, mechanism: Mechanism<String>) {
        match mechanism.kind() {
            Kind::Redirect => self.append_mechanism_of_redirect(mechanism),
            Kind::A => self.append_mechanism_of_a(mechanism),
            Kind::MX => self.append_mechanism_of_mx(mechanism),
            Kind::Include => self.append_mechanism_of_include(mechanism),
            Kind::Exists => self.append_mechanism_of_exists(mechanism),
            Kind::Ptr => self.append_mechanism_of_ptr(mechanism),
            Kind::All => self.append_mechanism_of_all(mechanism),
            _ => {}
        }
    }
    /// Appends the passed `Mechanism<IpNetwork>` to the SPF struct.
    ///
    /// # Example:
    /// ```
    /// use decon_spf::mechanism::{Qualifier, Mechanism};
    /// use decon_spf::Spf;
    /// let mut new_spf_record = Spf::new();
    /// new_spf_record.set_v1();
    /// new_spf_record.append_ip_mechanism(Mechanism::new_ip(Qualifier::Pass,
    ///                                 "203.32.160.0/23".parse().unwrap()));
    /// new_spf_record.append_mechanism(Mechanism::new_all(Qualifier::Pass));
    /// assert_eq!(new_spf_record.to_string(), "v=spf1 ip4:203.32.160.0/23 all".to_string());
    /// ```    
    pub fn append_ip_mechanism(&mut self, mechanism: Mechanism<IpNetwork>) {
        match mechanism.kind() {
            Kind::IpV4 => self.append_mechanism_of_ip4(mechanism),
            Kind::IpV6 => self.append_mechanism_of_ip6(mechanism),
            _ => {
                unreachable!()
            }
        }
    }
    /// # Note: Experimential
    /// *Do not use.*
    /// Very rudementary validation check.
    /// - Will fail if the length of `source` is more than MAX_SPF_STRING_LENGTH characters See:
    /// [`SourceLengthExceeded`](SpfError::SourceLengthExceeded)
    /// - Will fail if there are more than 10 DNS lookups. Looks are required for each `A`, `MX`
    /// , `Redirect`, and `Include` Mechanism. See: [`LookupLimitExceeded`](SpfError::LookupLimitExceeded)
    /// (This will change given new information)
    #[deprecated(note = "This is expected to be deprecated.")]
    pub fn try_validate(&mut self) -> Result<(), SpfError> {
        if self.from_src {
            if self.source.len() > MAX_SPF_STRING_LENGTH {
                return Err(SpfError::SourceLengthExceeded);
            } else if !self.was_parsed {
                return Err(SpfError::HasNotBeenParsed);
            };
        };
        // Rediect should be the only mechanism present. Any additional values are not permitted.
        if self.redirect().is_some() && self.all().is_some() {
            return Err(SpfError::RedirectWithAllMechanism);
        }
        if validate::check_lookup_count(self) > 10 {
            return Err(SpfError::LookupLimitExceeded);
        }
        self.is_valid = true;
        Ok(())
    }
    #[allow(dead_code)]
    fn validate(&mut self, rfc: SpfRfcStandard) -> Result<&Self, SpfError> {
        return match rfc {
            SpfRfcStandard::Rfc4408 => validate::validate_rfc4408(self),
        };
    }
    #[allow(dead_code)]
    fn validate_to_string(&mut self, rfc: SpfRfcStandard) -> SpfValidationResult {
        let res = match rfc {
            SpfRfcStandard::Rfc4408 => validate::validate_rfc4408(self),
        };
        match res {
            Ok(x) => SpfValidationResult::Valid(x),
            Err(x) => SpfValidationResult::InValid(x),
        }
    }

    fn build_spf_string(&self) -> String {
        let mut spf = String::new();
        spf.push_str(self.version());
        if self.a().is_some() {
            spf.push_str(helpers::build_spf_str(self.a()).as_str());
        };
        if self.mx().is_some() {
            spf.push_str(helpers::build_spf_str(self.mx()).as_str());
        };
        if self.includes().is_some() {
            spf.push_str(helpers::build_spf_str(self.includes()).as_str());
        }
        if self.ip4().is_some() {
            spf.push_str(helpers::build_spf_str_from_ip(self.ip4()).as_str());
        }
        if self.ip6().is_some() {
            spf.push_str(helpers::build_spf_str_from_ip(self.ip6()).as_str());
        }
        if self.exists().is_some() {
            spf.push_str(helpers::build_spf_str(self.exists()).as_str());
        }
        if self.ptr().is_some() {
            spf.push(' ');
            spf.push_str(self.ptr().unwrap().to_string().as_str());
        }
        if self.is_redirected {
            spf.push(' ');
            spf.push_str(self.redirect().unwrap().to_string().as_str());
        }
        // All can only be used if this is not a redirect.
        if !self.is_redirected && self.all().is_some() {
            spf.push(' ');
            spf.push_str(self.all().unwrap().to_string().as_str());
        }
        spf
    }
    /// Returns a new string representation of the spf record if possible.
    /// This does not use the `source` attribute.
    #[deprecated(
        since = "0.2.0",
        note = "This has been deprecated. Use to_string() instead."
    )]
    pub fn as_spf(&self) -> Result<String, SpfError> {
        unimplemented!("Spf struct now has a Display trait. Start using to_string()")
    }
    /// Returns a reference to the string stored in `source`
    pub fn source(&self) -> &String {
        // Source is set to "" by default.
        &self.source
    }
    /// True if there is a redirect present in the spf record.
    pub fn is_redirect(&self) -> bool {
        self.is_redirected
    }
    /// Returns a reference to the `Redirect` Mechanism
    pub fn redirect(&self) -> Option<&Mechanism<String>> {
        self.redirect.as_ref()
    }
    /// Returns a reference to the a `Vec` of `Mechanism<String>` for `Include`
    pub fn includes(&self) -> Option<&Vec<Mechanism<String>>> {
        self.include.as_ref()
    }
    /// Returns a reference to the a `Vec` of `Mechanism<String>` for `A`
    pub fn a(&self) -> Option<&Vec<Mechanism<String>>> {
        self.a.as_ref()
    }
    /// Returns a reference to the a `Vec` of `Mechanism<String>` for `MX`
    pub fn mx(&self) -> Option<&Vec<Mechanism<String>>> {
        self.mx.as_ref()
    }
    /// Returns a reference to the a `Vec` of `Mechanism<IpNetwork>` for `IP4`
    pub fn ip4(&self) -> Option<&Vec<Mechanism<IpNetwork>>> {
        self.ip4.as_ref()
    }
    /// Returns a reference to the a `Vec` of `Mechanism<IpNetwork>` for `IP6`
    pub fn ip6(&self) -> Option<&Vec<Mechanism<IpNetwork>>> {
        self.ip6.as_ref()
    }
    /// Returns a reference to the a `Vec` of `Mechanism<String>` for `Exists`
    pub fn exists(&self) -> Option<&Vec<Mechanism<String>>> {
        self.exists.as_ref()
    }
    /// Returns a reference to the a `Vec` of `Mechanism<String>` for `Ptr`
    pub fn ptr(&self) -> Option<&Mechanism<String>> {
        self.ptr.as_ref()
    }
    /// Returns a reference to `Mechanism<String>` for `All`
    pub fn all(&self) -> Option<&Mechanism<String>> {
        self.all.as_ref()
    }
    /// Return a reference to the list of domains that gave warnings.
    pub fn warnings(&self) -> Option<&Vec<String>> {
        self.warnings.as_ref()
    }
}
