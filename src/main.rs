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
use humantime::Duration;
use tokio_schedule::{every, Job};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Opt {
    #[arg(short, long)]
    region: Option<String>,

    #[arg(short = 'z', long)]
    hosted_zone_id: String,

    #[arg(short = 's', long)]
    record_set_name: String,

    #[arg(short, long, default_value = "5s")]
    interval: Duration,

    #[arg(short, long)]
    verbose: bool,
}

async fn check_and_update_ip(
    client: &aws_sdk_route53::Client,
    hosted_zone_id: &str,
    record_name: &str,
) -> Result<(), String> {
    let full_record_name = &format!("{record_name}.");

    let target_ip = get_my_ip().await?;
    println!("\nTarget ip {}", target_ip);

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
        interval,
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

    if verbose {
        println!("\nAttempting dns check/update on startup\n");
    }
    
    check_and_update_ip(&client, &hosted_zone_id, &record_set_name).await?;

    if verbose {
        println!("\nInitial run successful\n");
    }

    let interval_seconds = interval
        .as_secs()
        .try_into()
        .map_err(|_| {
            format!(
                "interval of {} seconds exceeds maximum value",
                interval.as_secs().to_string()
            )
        })
        .unwrap();

    if verbose {
        println!(
            "\nSchedule successful, refreshing every {} seconds",
            interval_seconds
        );
    }

    let scheduler = every(interval_seconds).seconds().perform(|| async {
        check_and_update_ip(&client, &hosted_zone_id, &record_set_name)
            .await
            .map(|_| println!("Update run successfully"))
            .unwrap_or_else(|err| println!("Update failed: {}", err))
    });

    scheduler.await;

    Ok(())
}
