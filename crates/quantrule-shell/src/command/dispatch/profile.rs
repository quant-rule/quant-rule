//! The profile command handlers.

use std::io::Write;

use quantrule_api::{CreateProfileRequest, ProfileId, ProfileSummary};
use serde_json::json;

use super::path::slug;
use crate::client::Client;
use crate::command::types::GlobalOptions;
use crate::error::RunError;
use crate::output;
use crate::session::Session;

pub(super) fn profile_list(
    session: &Session,
    client: &Client,
    globals: &GlobalOptions,
    stdout: &mut impl Write,
) -> Result<(), RunError> {
    let response = client.get("/profiles")?;
    let rows: Vec<ProfileSummary> = decode_json!(Vec<ProfileSummary>, response, "/profiles");
    let active = session.profile()?.to_string();
    output::emit(globals.json, json_view!(&rows), stdout, |writer| {
        output::profiles(writer, &rows, &active)
    })
}

pub(super) fn profile_create(
    client: &Client,
    globals: &GlobalOptions,
    stdout: &mut impl Write,
    name: &str,
    id: Option<&str>,
) -> Result<(), RunError> {
    let profile_id = match id {
        Some(id) => {
            ProfileId::parse(id).map_err(|error| RunError::Usage(format!("profile id {error}")))?
        }
        None => ProfileId::parse(&slug(name))
            .map_err(|error| RunError::Usage(format!("cannot derive a profile id: {error}")))?,
    };
    let request = CreateProfileRequest {
        profile_id,
        name: name.trim().to_owned(),
    };
    let body = serde_json::to_string(&request)
        .map_err(|error| RunError::decode("the profile request", error))?;
    let response = client.post_json("/profiles", &body)?;
    let profile: ProfileSummary = decode_json!(ProfileSummary, response, "/profiles");
    let created = response.status == 201;
    output::emit(globals.json, json_view!(&profile), stdout, |writer| {
        writeln!(writer, "created: {created}")?;
        output::profile(writer, &profile)
    })
}

pub(super) fn profile_use(
    session: &Session,
    client: &Client,
    globals: &GlobalOptions,
    stdout: &mut impl Write,
    profile_id: &str,
) -> Result<(), RunError> {
    let profile = ProfileId::parse(profile_id)
        .map_err(|error| RunError::Usage(format!("profile id {error}")))?;
    let response = client.get("/profiles")?;
    let rows: Vec<ProfileSummary> = decode_json!(Vec<ProfileSummary>, response, "/profiles");
    if !rows.iter().any(|row| row.profile_id == profile) {
        return Err(RunError::Api {
            code: String::from("profile.not_found"),
            message: format!("profile {profile} is unknown"),
        });
    }
    session.store_default(&profile)?;
    let view = json!({ "active": profile.to_string() });
    output::emit(globals.json, view, stdout, |writer| {
        output::active_profile(writer, profile.as_str())
    })
}
