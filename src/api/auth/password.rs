use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use std::sync::LazyLock;

pub(super) static DUMMY_PASSWORD_HASH: LazyLock<String> = LazyLock::new(|| {
    hash_password("code-terroir-dummy-password")
        .expect("the static dummy password must be hashable")
});

pub(super) fn hash_password(password: &str) -> Result<String, argon2::password_hash::Error> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
}

pub(super) fn verify_password(
    password: &str,
    password_hash: &str,
) -> Result<(), argon2::password_hash::Error> {
    let parsed_hash = PasswordHash::new(password_hash)?;
    Argon2::default().verify_password(password.as_bytes(), &parsed_hash)
}

#[cfg(test)]
mod tests {
    use super::{hash_password, verify_password};

    #[test]
    fn password_hash_roundtrip_works() {
        let hash = hash_password("secret123").expect("hash password");
        verify_password("secret123", &hash).expect("verify password");
        assert!(verify_password("bad", &hash).is_err());
    }
}
