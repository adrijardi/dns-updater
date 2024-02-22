use aws_sdk_route53::types::RrType::A;
use aws_sdk_route53::types::{
    Change, ChangeAction, ChangeBatch, ResourceRecord, ResourceRecordSet,
};
use aws_sdk_route53::Client;

pub async fn get_current_ip(
    client: &Client,
    hosted_zone_id: &str,
    full_record_name: &String,
) -> Result<String, String> {
    let record = client
        .list_resource_record_sets()
        .hosted_zone_id(hosted_zone_id)
        .set_start_record_name(Some(full_record_name.to_string()))
        .set_start_record_type(Some(A))
        .send()
        .await
        .map_err(|_| "Error retrieving records".to_string())?
        .resource_record_sets()
        .iter()
        .find(|r| r.name() == full_record_name && r.r#type == A)
        .ok_or("Record not found".to_string())?
        .to_owned();

    let current_ip = record
        .resource_records()
        .first()
        .map(|r| r.value().trim().to_string());

    current_ip.ok_or("Ip not found".to_string())
}

pub async fn update_ip(
    client: &Client,
    hosted_zone_id: &str,
    full_record_name: &String,
    target_ip: &str,
) -> Result<(), String> {
    client
        .change_resource_record_sets()
        .hosted_zone_id(hosted_zone_id)
        .change_batch(
            ChangeBatch::builder()
                .changes(
                    Change::builder()
                        .action(ChangeAction::Upsert)
                        .resource_record_set(
                            ResourceRecordSet::builder()
                                .name(full_record_name)
                                .r#type(A)
                                .ttl(300)
                                .resource_records(
                                    ResourceRecord::builder().value(target_ip).build().unwrap(),
                                )
                                .build()
                                .unwrap(),
                        )
                        .build()
                        .unwrap(),
                )
                .build()
                .unwrap(),
        )
        .send()
        .await
        .map_err(|_| "Failed to upsert ip address")?;
    Ok(())
}
