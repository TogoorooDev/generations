use crate::prelude::*;

pub fn parse_addr_or_name(account: &Account, s: &str) -> Option<SufecAddr> {
	Some(if let Some(c) = account.contacts.iter().find(|c| c.name == s) {
		c.addr.clone()
	} else if let Ok(addr) = SufecAddr::try_from(s) {
		addr
	} else {
		return None
	})
}
pub fn display_addr_or_name(account: &Account, addr: &SufecAddr) -> String {
	if addr.id == account.account.addr.id {
		return "me".to_string()
	}
	if let Some(c) = account.contacts.iter().find(|c| c.addr.id == addr.id) {
		return c.name.clone()
	}
	addr.to_string()
}

#[macro_export]
macro_rules! require_some {
    ($option:expr) => {
		match $option {
			Some(v) => v,
			None => return,
		}
    };
}
