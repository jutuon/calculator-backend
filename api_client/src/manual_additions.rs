use crate::{
    apis::{configuration, Error, ResponseContent},
    models::AccountIdLight,
};

impl Copy for AccountIdLight {}

impl AccountIdLight {
    pub fn to_string(&self) -> String {
        self.account_id.hyphenated().to_string()
    }
}

pub async fn api_available(configuration: &configuration::Configuration) -> Result<(), ()> {
    let local_var_configuration = configuration;

    let local_var_client = &local_var_configuration.client;

    let local_var_uri_str = format!("{}/", local_var_configuration.base_path,);
    let mut local_var_req_builder =
        local_var_client.request(reqwest::Method::PUT, local_var_uri_str.as_str());

    if let Some(ref local_var_user_agent) = local_var_configuration.user_agent {
        local_var_req_builder =
            local_var_req_builder.header(reqwest::header::USER_AGENT, local_var_user_agent.clone());
    }
    if let Some(ref local_var_apikey) = local_var_configuration.api_key {
        let local_var_key = local_var_apikey.key.clone();
        let local_var_value = match local_var_apikey.prefix {
            Some(ref local_var_prefix) => format!("{} {}", local_var_prefix, local_var_key),
            None => local_var_key,
        };
        local_var_req_builder = local_var_req_builder.header("x-api-key", local_var_value);
    };

    let local_var_req = local_var_req_builder.build().map_err(|_| ())?;
    let _ = local_var_client
        .execute(local_var_req)
        .await
        .map_err(|_| ())?;

    Ok(())
}
