//! This module allows you to deconstruct an exiting SPF DNS Record into its
//! constituant parts.  
//! It is not intended to validate the spf record.

pub mod kinds;
pub mod mechanism;
pub mod qualifier;
#[doc(hidden)]
mod tests;

use crate::spf::mechanism::SpfMechanism;
use crate::spf::qualifier::Qualifier;
use ipnetwork::IpNetwork;
use regex::Regex;

#[derive(Default, Debug)]
pub struct Spf {
    source: String,
    from_src: bool,
    include: Option<Vec<SpfMechanism<String>>>,
    redirect: Option<SpfMechanism<String>>,
    is_redirected: bool,
    a: Option<Vec<SpfMechanism<String>>>,
    mx: Option<Vec<SpfMechanism<String>>>,
    ip4: Option<Vec<SpfMechanism<IpNetwork>>>,
    ip6: Option<Vec<SpfMechanism<IpNetwork>>>,
    all: Option<SpfMechanism<String>>,
}

impl Spf {
    /// Create a new Spf with the provided `str`
    ///
    /// # Example
    ///
    /// ```
    /// use decon_spf::spf::Spf;
    /// let source_str = "v=spf1 redirect=_spf.example.com";
    /// let spf = Spf::from_str(&source_str.to_string());
    /// ```
    ///
    pub fn from_str(str: &String) -> Self {
        Self {
            source: str.clone(),
            from_src: true,
            include: None,
            redirect: None,
            is_redirected: false,
            a: None,
            mx: None,
            ip4: None,
            ip6: None,
            all: None,
        }
    }
    pub fn new() -> Self {
        Self {
            source: String::new(),
            from_src: false,
            include: None,
            redirect: None,
            is_redirected: false,
            a: None,
            mx: None,
            ip4: None,
            ip6: None,
            all: None,
        }
    }
    /// Parse the contents of `source` and populate the internal structure of `Spf`
    pub fn parse(&mut self) {
        // initialises required variables.
        let records = self.source.split_whitespace();
        let mut vec_of_includes: Vec<SpfMechanism<String>> = Vec::new();
        let mut vec_of_ip4: Vec<SpfMechanism<IpNetwork>> = Vec::new();
        let mut vec_of_ip6: Vec<SpfMechanism<IpNetwork>> = Vec::new();
        let mut vec_of_a: Vec<SpfMechanism<String>> = Vec::new();
        let mut vec_of_mx: Vec<SpfMechanism<String>> = Vec::new();
        for record in records {
            // Make this lazy.
            let a_pattern =
                Regex::new(r"^(?P<qualifier>[+?~-])?(?P<mechanism>a(?:[:/]{0,1}.+)?)").unwrap();
            let mx_pattern =
                Regex::new(r"^(?P<qualifier>[+?~-])?(?P<mechanism>mx(?:[:/]{0,1}.+)?)").unwrap();
            if record.contains("redirect=") {
                // Match a redirect
                let items = record.rsplit("=");
                for item in items {
                    self.redirect = Some(SpfMechanism::new_redirect(
                        Qualifier::Pass,
                        item.to_string(),
                    ));
                    break;
                }
                self.is_redirected = true;
            } else if record.contains("include:") {
                // Match an include
                let qualifier_and_modified_str = return_and_remove_qualifier(record, 'i');
                for item in record.rsplit(":") {
                    vec_of_includes.push(SpfMechanism::new_include(
                        qualifier_and_modified_str.0,
                        item.to_string(),
                    ));
                    break; // skip the 'include:'
                }
            } else if record.contains("ip4:") {
                // Match an ip4
                let qualifier_and_modified_str = return_and_remove_qualifier(record, 'i');
                if let Some(raw_ip4) = qualifier_and_modified_str.1.strip_prefix("ip4:") {
                    let network = SpfMechanism::new_ip4(
                        qualifier_and_modified_str.0,
                        raw_ip4.parse().unwrap(),
                    );
                    vec_of_ip4.push(network);
                }
            } else if record.contains("ip6:") {
                // Match an ip6
                let qualifier_and_modified_str = return_and_remove_qualifier(record, 'i');
                if let Some(raw_ip6) = qualifier_and_modified_str.1.strip_prefix("ip6:") {
                    let network = SpfMechanism::new_ip6(
                        qualifier_and_modified_str.0,
                        raw_ip6.parse().unwrap(),
                    );
                    vec_of_ip6.push(network);
                }
            } else if record.ends_with("all") {
                // deal with all if present
                self.all = Some(SpfMechanism::new_all(
                    return_and_remove_qualifier(record, 'a').0,
                ))
            } else if let Some(a_mechanism) =
                capture_matches(a_pattern, record, kinds::MechanismKind::A)
            {
                vec_of_a.push(a_mechanism);
            } else if let Some(mx_mechanism) =
                capture_matches(mx_pattern, record, kinds::MechanismKind::MX)
            {
                vec_of_mx.push(mx_mechanism);
            }
        }
        if !vec_of_includes.is_empty() {
            self.include = Some(vec_of_includes);
        };
        if !vec_of_ip4.is_empty() {
            self.ip4 = Some(vec_of_ip4);
        };
        if !vec_of_ip6.is_empty() {
            self.ip6 = Some(vec_of_ip6);
        };
        if !vec_of_a.is_empty() {
            self.a = Some(vec_of_a);
        }
        if !vec_of_mx.is_empty() {
            self.mx = Some(vec_of_mx);
        }
    }

    pub fn is_valid(&self) -> bool {
        if self.from_src {
            if self.include.is_some() {
                if self.include.as_ref().unwrap().len() > 10 {
                    return false;
                } else {
                    true
                }
            } else {
                true
            }
        } else {
            true
        }
    }
    pub fn source(&self) -> &String {
        &self.source
    }
    pub fn includes(&self) -> Option<&Vec<SpfMechanism<String>>> {
        self.include.as_ref()
    }
    pub fn list_includes(&self) {
        match &self.include {
            None => println!("There are no include elements"),
            Some(elements) => {
                println!("Include Mechanisms:");
                for element in elements {
                    println!("{}", element.string());
                }
            }
        }
    }
    pub fn a(&self) -> Option<&Vec<SpfMechanism<String>>> {
        self.a.as_ref()
    }
    pub fn mx(&self) -> Option<&Vec<SpfMechanism<String>>> {
        self.mx.as_ref()
    }
    pub fn ip4(&self) -> Option<&Vec<SpfMechanism<IpNetwork>>> {
        self.ip4.as_ref()
    }
    pub fn ip4_networks(&self) {
        match &self.ip4 {
            None => println!("There are no ip4 networks"),
            Some(record) => {
                println!("List of ip4 networks/hosts:");
                for item in record {
                    println!("{}", item.raw());
                    print!("Network: {}", item.as_network().network());
                    println!(" Subnet: {}", item.as_network().prefix());
                }
            }
        }
    }
    pub fn ip4_mechanisms(&self) {
        match &self.ip4 {
            None => println!("There are no ip4 spf records."),
            Some(records) => {
                println!("\nList of ip4 mechanisms:");
                for record in records {
                    println!("{}", record.string())
                }
            }
        }
    }
    pub fn ip6(&self) -> Option<&Vec<SpfMechanism<IpNetwork>>> {
        self.ip6.as_ref()
    }
    pub fn ip6_networks(&self) {
        match &self.ip6 {
            None => println!("There are no ip6 networks"),
            Some(record) => {
                println!("List of ip6 networks/hosts:");
                for item in record {
                    println!("{}", item.raw());
                    print!("Network: {}", item.mechanism().network());
                    println!(" Subnet: {}", item.mechanism().prefix());
                }
            }
        }
    }
    pub fn ip6_mechanisms(&self) {
        match &self.ip6 {
            None => println!("There are no ip6 spf records."),
            Some(records) => {
                println!("\nList of ip6 mechanisms:");
                for record in records {
                    println!("{}", record.string())
                }
            }
        }
    }

    pub fn is_redirect(&self) -> bool {
        self.is_redirected
    }
    pub fn redirect(&self) -> Option<&SpfMechanism<String>> {
        self.redirect.as_ref()
    }
    //pub fn redirect_as_mechanism(&self) -> String {
    //    if self.redirect.is_some() {
    //        self.redirect.as_ref().unwrap().as_mechanism()
    //    } else {
    //        "".to_string()
    //    }
    //}
    pub fn all(&self) -> Option<&SpfMechanism<String>> {
        self.all.as_ref()
    }
}
fn char_to_qualifier(c: char) -> Qualifier {
    match c {
        '+' => return Qualifier::Pass,
        '-' => return Qualifier::Fail,
        '~' => return Qualifier::SoftFail,
        '?' => return Qualifier::Neutral,
        _ => return Qualifier::Pass, // This should probably be Neutral
    }
}
#[doc(hidden)]
// Check if the initial character in the string `record` matches `c`
// If they do no match then return the initial character
// if c matches first character of record, we can `+`, a blank modiifer equates to `+`
fn return_and_remove_qualifier(record: &str, c: char) -> (Qualifier, &str) {
    // Returns a tuple of (qualifier, &str)
    // &str will have had the qualifier character removed if it existed. The &str will be unchanged
    // if the qualifier was not present
    if c != record.chars().nth(0).unwrap() {
        // qualifier exists. return tuple of qualifier and `record` with qualifier removed.
        (
            char_to_qualifier(record.chars().nth(0).unwrap()),
            remove_qualifier(record),
        )
    } else {
        // qualifier does not exist, default to `+` and return unmodified `record`
        (Qualifier::Pass, record)
    }
}
#[test]
fn test_return_and_remove_qualifier_no_qualifier() {
    let source = "no prefix";
    let (c, new_str) = return_and_remove_qualifier(source, 'n');
    assert_eq!(Qualifier::Pass, c);
    assert_eq!(source, new_str);
}
#[test]
fn test_return_and_remove_qualifier_pass() {
    let source = "+prefix";
    let (c, new_str) = return_and_remove_qualifier(source, 'n');
    assert_eq!(Qualifier::Pass, c);
    assert_eq!("prefix", new_str);
}
#[test]
fn test_return_and_remove_qualifier_fail() {
    let source = "-prefix";
    let (c, new_str) = return_and_remove_qualifier(source, 'n');
    assert_eq!(Qualifier::Fail, c);
    assert_eq!("prefix", new_str);
}
#[test]
fn test_return_and_remove_qualifier_softfail() {
    let source = "~prefix";
    let (c, new_str) = return_and_remove_qualifier(source, 'n');
    assert_eq!(Qualifier::SoftFail, c);
    assert_eq!("prefix", new_str);
}
#[test]
fn test_return_and_remove_qualifier_neutral() {
    let source = "?prefix";
    let (c, new_str) = return_and_remove_qualifier(source, 'n');
    assert_eq!(Qualifier::Neutral, c);
    assert_eq!("prefix", new_str);
}
#[doc(hidden)]
fn remove_qualifier(record: &str) -> &str {
    // Remove leading (+,-,~,?) character and return an updated str
    let mut chars = record.chars();
    chars.next();
    chars.as_str()
}
#[test]
fn test_remove_qualifier() {
    let test_str = "abc";
    let result = remove_qualifier(test_str);
    assert_eq!(result, "bc");
}
#[doc(hidden)]
fn capture_matches(
    pattern: Regex,
    string: &str,
    kind: kinds::MechanismKind,
) -> Option<SpfMechanism<String>> {
    let caps = pattern.captures(string);
    let q: char;
    let mut q2: Qualifier = Qualifier::Pass;
    let m: String;
    match caps {
        None => return None,
        Some(caps) => {
            // There was a match
            if caps.name("qualifier").is_some() {
                q = caps
                    .name("qualifier")
                    .unwrap()
                    .as_str()
                    .chars()
                    .nth(0)
                    .unwrap();
                q2 = char_to_qualifier(q);
            };
            m = caps.name("mechanism").unwrap().as_str().to_string();
            let mechanism = SpfMechanism::new(kind, q2, (*m).to_string());
            Some(mechanism)
        }
    }
}
#[test]
fn test_match_on_a_only() {
    let string = "a";
    let pattern = Regex::new(r"^(?P<qualifier>[+?~-])?(?P<mechanism>a(?:[:/]{0,1}.+)?)").unwrap();
    let option_test: Option<SpfMechanism<String>>;

    option_test = capture_matches(pattern, &string, kinds::MechanismKind::A);

    let test = option_test.unwrap();
    assert_eq!(test.is_pass(), true);
    assert_eq!(test.raw(), "a");
    assert_eq!(test.string(), "a");
}
#[test]
fn test_match_on_a_colon() {
    let string = "-a:example.com";
    let pattern = Regex::new(r"^(?P<qualifier>[+?~-])?(?P<mechanism>a(?:[:/]{0,1}.+)?)").unwrap();
    let option_test: Option<SpfMechanism<String>>;

    option_test = capture_matches(pattern, &string, kinds::MechanismKind::A);

    let test = option_test.unwrap();
    assert_eq!(test.is_fail(), true);
    assert_eq!(test.raw(), "a:example.com");
    assert_eq!(test.string(), "-a:example.com");
}
#[test]
fn test_match_on_a_slash() {
    let string = "~a/24";
    let pattern = Regex::new(r"^(?P<qualifier>[+?~-])?(?P<mechanism>a(?:[:/]{0,1}.+)?)").unwrap();
    let option_test: Option<SpfMechanism<String>>;

    option_test = capture_matches(pattern, &string, kinds::MechanismKind::A);

    let test = option_test.unwrap();
    assert_eq!(test.is_softfail(), true);
    assert_eq!(test.raw(), "a/24");
    assert_eq!(test.string(), "~a/24");
}
#[test]
fn test_match_on_a_colon_slash() {
    let string = "+a:example.com/24";
    let pattern = Regex::new(r"^(?P<qualifier>[+?~-])?(?P<mechanism>a(?:[:/]{0,1}.+)?)").unwrap();
    let option_test: Option<SpfMechanism<String>>;

    option_test = capture_matches(pattern, &string, kinds::MechanismKind::A);

    let test = option_test.unwrap();
    assert_eq!(test.is_pass(), true);
    assert_eq!(test.raw(), "a:example.com/24");
    assert_eq!(test.string(), "a:example.com/24");
    //assert!(test.kind.is_a());
}
// MX
#[test]
fn test_match_on_mx_only() {
    let string = "mx";
    let pattern = Regex::new(r"^(?P<qualifier>[+?~-])?(?P<mechanism>mx(?:[:/]{0,1}.+)?)").unwrap();
    let option_test: Option<SpfMechanism<String>>;

    option_test = capture_matches(pattern, &string, kinds::MechanismKind::MX);

    let test = option_test.unwrap();
    assert_eq!(test.is_pass(), true);
    assert_eq!(test.raw(), "mx");
    assert_eq!(test.string(), "mx");
}
#[test]
fn test_match_on_mx_colon() {
    let string = "-mx:example.com";
    let pattern = Regex::new(r"^(?P<qualifier>[+?~-])?(?P<mechanism>mx(?:[:/]{0,1}.+)?)").unwrap();
    let option_test: Option<SpfMechanism<String>>;

    option_test = capture_matches(pattern, &string, kinds::MechanismKind::MX);

    let test = option_test.unwrap();
    assert_eq!(test.is_fail(), true);
    assert_eq!(test.raw(), "mx:example.com");
    assert_eq!(test.string(), "-mx:example.com");
}
#[test]
fn test_match_on_mx_slash() {
    let string = "~mx/24";
    let pattern = Regex::new(r"^(?P<qualifier>[+?~-])?(?P<mechanism>mx(?:[:/]{0,1}.+)?)").unwrap();
    let option_test: Option<SpfMechanism<String>>;

    option_test = capture_matches(pattern, &string, kinds::MechanismKind::MX);

    let test = option_test.unwrap();
    assert_eq!(test.is_softfail(), true);
    assert_eq!(test.raw(), "mx/24");
    assert_eq!(test.string(), "~mx/24");
}
#[test]
fn test_match_on_mx_colon_slash() {
    let string = "+mx:example.com/24";
    let pattern = Regex::new(r"^(?P<qualifier>[+?~-])?(?P<mechanism>mx(?:[:/]{0,1}.+)?)").unwrap();
    let option_test: Option<SpfMechanism<String>>;

    option_test = capture_matches(pattern, &string, kinds::MechanismKind::MX);

    let test = option_test.unwrap();
    assert_eq!(test.is_pass(), true);
    assert_eq!(test.raw(), "mx:example.com/24");
    assert_eq!(test.string(), "mx:example.com/24");
}
