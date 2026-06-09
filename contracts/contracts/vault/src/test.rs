#![cfg(test)]

use super::*;

#[test]
fn version_returns_one() {
    assert_eq!(Vault::version(), 1);
}
