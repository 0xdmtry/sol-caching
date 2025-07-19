use solana_client::client_error::{ClientError, ClientErrorKind};

pub trait IsTransient: std::fmt::Debug {
    fn is_transient(&self) -> bool;
}

impl IsTransient for ClientError {
    fn is_transient(&self) -> bool {
        match &self.kind {
            // Cases considered transient
            // Presently, just one; but there could be added additional
            ClientErrorKind::Reqwest(_) | ClientErrorKind::Io(_) => true,

            // All other cases
            _ => false,
        }
    }
}
