use lazy_static::lazy_static;

pub const VALIDATOR_AS_BUFFER: &'static [u8] = "Validator".as_bytes();
pub const BUNDLR_AS_BUFFER: &[u8] = "Bundlr".as_bytes();

lazy_static! {
    static ref VALIDATOR_ADDRESS: String = {
        let key = serde_json::from_slice::<<::(include_bytes!("../wallet.json")).unwrap();

        String::default()
    };
}