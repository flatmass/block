use crate::error::Error;

pub fn verify_node_name(name: &str) -> Result<(), Error> {
    if name.len() <= 256 {
        Ok(())
    } else {
        Error::too_long_param("node_name").ok()
    }
}

pub fn verify_filename(name: &str) -> Result<(), Error> {
    if name.len() > 256 {
        Error::too_long_param("filename").ok()
    } else if name.chars().any(|c| matches!(c, '\0' | ':' | '/')) {
        Error::unexpected_param_value("filename").ok()
    } else {
        Ok(())
    }
}
