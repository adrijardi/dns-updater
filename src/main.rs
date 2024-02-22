#![allow(clippy::result_large_err)]

mod dns_api;
mod my_ip_api;

use crate::{
    dns_api::{get_current_ip, update_ip},
    my_ip_api::get_my_ip,
};
use aws_config::meta::region::RegionProviderChain;
use aws_sdk_route53::{config::Region, Client};
use clap::Parser;
use tokio_schedule::{every, Job};

#[derive(Debug, Parser)]
struct Opt {
    /// The AWS Region.
    #[structopt(short, long)]
    region: Option<String>,

    /// The hosted zone id
    #[structopt(short = 'z', long)]
    hosted_zone_id: String,

    /// The record set name
    #[structopt(short = 's', long)]
    record_set_name: String,

    /// Whether to display additional runtime information.
    #[structopt(short, long)]
    verbose: bool,
}

async fn check_and_update_ip(
    client: &aws_sdk_route53::Client,
    hosted_zone_id: &str,
    record_name: &str,
) -> Result<(), String> {
    let full_record_name = &format!("{record_name}.");

    let target_ip = get_my_ip().await?;
    println!();
    println!("Target ip {}", target_ip);

    let current_ip = get_current_ip(client, hosted_zone_id, full_record_name).await?;

    println!("Current ip: {}", current_ip);

    if current_ip == target_ip {
        println!("Ips matching");
    } else {
        println!("Ips not matching, updating");

        update_ip(client, hosted_zone_id, full_record_name, &target_ip).await?;
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), String> {
    tracing_subscriber::fmt::init();

    let Opt {
        region,
        hosted_zone_id,
        record_set_name,
        verbose,
    } = Opt::parse();

    let region_provider = RegionProviderChain::first_try(region.map(Region::new))
        .or_default_provider()
        .or_else(Region::new("us-west-2"));

    if verbose {
        println!("Dns-Updater");
        println!(
            "Region: {}",
            region_provider.region().await.unwrap().as_ref()
        );
        println!();
    }

    let shared_config = aws_config::from_env().region(region_provider).load().await;
    let client = Client::new(&shared_config);

    check_and_update_ip(&client, &hosted_zone_id, &record_set_name)
        .await?;

    println!();
    println!("Startup successful");

    let scheduler = every(5).minutes().perform(|| async {
        check_and_update_ip(&client, &hosted_zone_id, &record_set_name)
        .await
        .map(|_| println!("Update run successfully"))
        .unwrap_or_else(|err| println!("Update failed: {}", err))
    });

    scheduler.await;

    Ok(())
}
