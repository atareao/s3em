#[derive(Clone, Debug)]
pub struct Config {
    pub server_host: String,
    pub server_port: u16,
    pub database_url: String,
    pub s3_endpoint: String,
    pub s3_region: String,
    pub s3_bucket: String,
    pub s3_access_key: String,
    pub s3_secret_key: String,
    pub s3_path_style: bool,
    pub jwt_secret: String,
    pub master_api_key: String,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            server_host: std::env::var("SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".into()),
            server_port: std::env::var("SERVER_PORT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(4004),
            database_url: std::env::var("DATABASE_URL").unwrap_or_else(|_| "s3manager.db".into()),
            s3_endpoint: std::env::var("S3_ENDPOINT").unwrap_or_else(|_| "http://localhost:9000".into()),
            s3_region: std::env::var("S3_REGION").unwrap_or_else(|_| "us-east-1".into()),
            s3_bucket: std::env::var("S3_BUCKET").unwrap_or_else(|_| "s3manager".into()),
            s3_access_key: std::env::var("S3_ACCESS_KEY").unwrap_or_else(|_| "minioadmin".into()),
            s3_secret_key: std::env::var("S3_SECRET_KEY").unwrap_or_else(|_| "minioadmin".into()),
            s3_path_style: std::env::var("S3_PATH_STYLE")
                .ok()
                .map(|v| v == "true" || v == "1")
                .unwrap_or(true),
            jwt_secret: std::env::var("JWT_SECRET")
                .unwrap_or_else(|_| "change-me-in-production".into()),
            master_api_key: std::env::var("MASTER_API_KEY")
                .unwrap_or_else(|_| "dev-key-123".into()),
        }
    }

    pub fn server_addr(&self) -> String {
        format!("{}:{}", self.server_host, self.server_port)
    }
}