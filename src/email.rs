use std::time::SystemTime;

pub struct EmailService {
    from_email: String,
    auth_token: String,
    enabled: bool,
}

impl EmailService {
    pub fn new(from_email: String, auth_token: String) -> Self {
        let enabled = !auth_token.is_empty() && auth_token != "xxxxxxxxxxxxxxxx";
        Self {
            from_email,
            auth_token,
            enabled,
        }
    }

    pub fn send_verification(&self, to_email: &str, code: &str) -> Result<(), String> {
        if !self.enabled {
            println!("Email service not configured - would send to {}: code={}", to_email, code);
            return Ok(());
        }

        println!("Sending verification code {} to {}", code, to_email);
        Ok(())
    }
}

impl Default for EmailService {
    fn default() -> Self {
        Self {
            from_email: "TDR_Group@foxmail.com".to_string(),
            auth_token: String::new(),
            enabled: false,
        }
    }
}

pub fn generate_verification_code() -> String {
    let seed = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(42);
    let code = 100000 + (seed % 900000) as u32;
    code.to_string()
}