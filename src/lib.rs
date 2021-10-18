#![forbid(unsafe_code)]
#![warn(missing_docs)]
//! This crate is responsible for providing tools to access and modify information about spf records.  
//! Provides methods for building spf records programmatically.  
//!
//! Deconstructing an existing spf record into its corresponding components.
//! ========================================================================
//!```rust
//! use decon_spf::spf::Spf;
//! let test_str = "v=spf1 a mx ~all";
//! // Parsing updates information in the Spf Struct; it needs to be mutable.
//! let mut spf = Spf::from_str(test_str);
//! // Parse test_str and populate Spf Struct.  
//! // Parse returns a Result<(), SpfErrorType> allowing for some error checking.  
//! let check = spf.parse();
//! assert_eq!(check.is_ok(), true);
//! // spf should be of v_1 form
//! assert_eq!(spf.is_v1(), true);
//! // There should be no ip4 or ip6 data
//! assert_eq!(spf.ip4().is_none(), true);
//! assert_eq!(spf.ip6().is_none(), true);
//! // Check that 'A' and 'MX' both have records.
//! assert_eq!(spf.a().is_some(), true);
//! assert_eq!(spf.mx().is_some(), true);
//! assert_eq!(spf.all().is_some(), true);
//! // Check that 'All' is a soft fail denoted by the use of '~'
//! assert_eq!(spf.all().unwrap().is_softfail(), true);
//! // Generate the spf record based on the content of the Spf struct.
//! // Does not use the original source string.
//! // # note: `Display` has been implemented for Spf so we could usually make the call
//! // as follows.
//! println!("{}", spf);
//! assert_eq!(spf.to_string(), "v=spf1 a mx ~all");
//!```
//!
//! Build an Spf Struct programmatically.
//! =====================================
//!
//!```rust
//! use decon_spf::spf::Spf;
//! use decon_spf::mechanism::{Qualifier, Kind, Mechanism};
//! let mut spf1 = Spf::new();
//! spf1.set_v1();
//! spf1.append_ip_mechanism(Mechanism::new_ip(Qualifier::Pass,
//!                                            "203.32.160.0/32".parse().unwrap()));
//!
//! assert_eq!(spf1.to_string(), "v=spf1 ip4:203.32.160.0/32");
//! println!("New spf 1: >{}<", spf1);
//!
//! let mut spf2 = Spf::new();
//! spf2.set_v1();
//! let ip = "203.32.166.0/24".parse().unwrap();
//! spf2.append_ip_mechanism(Mechanism::new_ip(Qualifier::Pass, ip));
//!
//! println!("\nNew spf 2: >{}<", spf2);
//! assert_eq!(spf2.to_string(), "v=spf1 ip4:203.32.166.0/24");
//! println!("Add mx to spf2");
//! spf2.append_mechanism(Mechanism::new_mx_without_mechanism(Qualifier::Pass));
//!
//! assert_eq!(spf2.to_string(), "v=spf1 mx ip4:203.32.166.0/24");
//! println!("Altered spf 2: >{}<", spf2);
//! println!("Clear mx from spf2");
//! spf2.clear_mechanism(Kind::MX);
//! assert_eq!(spf2.to_string(), "v=spf1 ip4:203.32.166.0/24");
//! println!("Altered spf 2: >{}<", spf2);
//!
//! let mut spf3 = Spf::new();
//! spf3.set_v2_pra();
//! spf3.append_mechanism(Mechanism::new_a_without_mechanism(Qualifier::Pass));
//! spf3.append_mechanism(Mechanism::new_all(Qualifier::Neutral));
//!
//! assert_eq!(spf3.to_string(), "spf2.0/pra a ?all");
//! println!("\nNew spf 3: >{}<", spf3);
//! println!("Change spf3 all to Fail");
//! spf3.append_mechanism(Mechanism::new_all(Qualifier::Fail));
//! assert_eq!(spf3.to_string(), "spf2.0/pra a -all");
//! println!("Altered spf 3: >{}<", spf3);
//!```
//!
//!
//! For a list of supported *Modifiers* and *Mechanism*. See [mechanism::Kind](mechanism::Kind)  
//!
//! The ability to check what version of the Spf record was specified as:
//!
//! - *v=spf1* or *spf2.0/* using `is_v1()` or `is_v2()`
//!
//! Check if the spf record is a *redirect*, *A*, *MX* and more
//!
//! - [is_redirect()](mechanism::Kind::is_redirect()), [redirect()](spf::Spf::redirect())
//! - `is_a()`, `a()`
//!
//! For any given mechanism we can:
//! - check its *Qualifier* status
//!     - `is_pass()`, `is_fail()`, `is_softfail()`, `is_neutral()`
//!
//! - Check its *Kind*
//!     - `kind().is_a()`, `kind().is_mx()` and more.
//!
//! For **IP4/6** we can access it as an [IpNetwork](mechanism::Mechanism::as_network())
//! as well as access its [String](mechanism::Mechanism<IpNetwork>::string()) representation.
//!
//! See [mechanism::Mechanism].
//!
mod helpers;
pub mod mechanism;
pub mod spf;
