use croncat::{logging::{self}, errors::Report, tokio, grpc};

mod cli;

#[tokio::main]
async fn main() -> Result<(), Report> {
    logging::setup()?;

    cli::print_banner();
    
    let _client = grpc::connect().await?;
    
    Ok(())
}

