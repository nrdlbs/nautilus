use clap::Parser;
use fastcrypto::{ed25519::Ed25519KeyPair, traits::KeyPair};
use nsm_api::api::{Request as NsmRequest, Response as NsmResponse};
use nsm_api::driver;
use serde_bytes::ByteBuf;

#[derive(Parser)]
#[command(name = "get-attestation")]
#[command(about = "Lấy attestation trực tiếp từ NSM driver mà không cần endpoint")]
struct Cli {
    /// Public key để include trong attestation (hex format)
    #[arg(long)]
    public_key: Option<String>,
    
    /// Nonce để include trong attestation (hex format) 
    #[arg(long)]
    nonce: Option<String>,
    
    /// User data để include trong attestation (hex format)
    #[arg(long)]
    user_data: Option<String>,
    
    /// Tạo keypair mới và sử dụng public key của nó
    #[arg(long)]
    generate_keypair: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    
    // Xử lý public key
    let public_key = if cli.generate_keypair {
        let kp = Ed25519KeyPair::generate(&mut rand::thread_rng());
        println!("Generated keypair. Public key: {}", hex::encode(kp.public().as_bytes()));
        Some(ByteBuf::from(kp.public().as_bytes().to_vec()))
    } else if let Some(pk_hex) = cli.public_key {
        let pk_bytes = hex::decode(pk_hex)?;
        Some(ByteBuf::from(pk_bytes))
    } else {
        None
    };
    
    // Xử lý nonce
    let nonce = if let Some(nonce_hex) = cli.nonce {
        let nonce_bytes = hex::decode(nonce_hex)?;
        Some(ByteBuf::from(nonce_bytes))
    } else {
        None
    };
    
    // Xử lý user data
    let user_data = if let Some(data_hex) = cli.user_data {
        let data_bytes = hex::decode(data_hex)?;
        Some(ByteBuf::from(data_bytes))
    } else {
        None
    };
    
    println!("Khởi tạo NSM driver...");
    let fd = driver::nsm_init();
    
    println!("Gửi attestation request...");
    let request = NsmRequest::Attestation {
        user_data,
        nonce,
        public_key,
    };
    
    let response = driver::nsm_process_request(fd, request);
    
    match response {
        NsmResponse::Attestation { document } => {
            driver::nsm_exit(fd);
            println!("Thành công! Attestation document:");
            println!("{}", hex::encode(&document));
        }
        _ => {
            driver::nsm_exit(fd);
            eprintln!("Lỗi: Phản hồi không mong đợi từ NSM");
            std::process::exit(1);
        }
    }
    
    Ok(())
}
