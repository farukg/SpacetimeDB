use super::*;
use bytes::Bytes;
use futures_util::{Stream, StreamExt, TryStreamExt};
use http_body_util::BodyExt;
use st_http::{HttpPolicyError, RequestWithPolicy};

impl InstanceEnv {
    pub async fn http_request_with_policy(
        &mut self,
        request: RequestWithPolicy,
        body: Bytes,
    ) -> Result<(st_http::Response, Bytes), HttpPolicyError> {
        if self.in_tx() {
            return Err(HttpPolicyError::WouldBlockTransaction);
        }
        let identity = *self.database_identity();
        DB_METRICS
            .procedure_num_http_requests
            .with_label_values(&identity)
            .inc();
        DB_METRICS
            .procedure_http_request_size_bytes
            .with_label_values(&identity)
            .inc_by((request.request.size_in_bytes() + body.len()) as _);
        let _in_progress = DB_METRICS
            .procedure_num_in_progress_http_requests
            .with_label_values(&identity)
            .inc_scope();
        let result = tokio::spawn(execute(request, body))
            .await
            .map_err(|_| HttpPolicyError::HostFailure)?;
        match &result {
            Ok((response, body)) => {
                DB_METRICS
                    .procedure_http_response_size_bytes
                    .with_label_values(&identity)
                    .inc_by((response.size_in_bytes() + body.len()) as _);
            }
            Err(HttpPolicyError::Timeout) => {
                DB_METRICS
                    .procedure_num_timeout_http_requests
                    .with_label_values(&identity)
                    .inc();
            }
            Err(_) => {
                DB_METRICS
                    .procedure_num_failed_http_requests
                    .with_label_values(&identity)
                    .inc();
            }
        }
        result
    }
}

pub(super) fn legacy_client_builder() -> reqwest::ClientBuilder {
    let redirect_policy = reqwest::redirect::Policy::custom(|attempt| {
        if is_blocked_ip_literal(attempt.url()) {
            attempt.error(BLOCKED_HTTP_ADDRESS_ERROR)
        } else {
            reqwest::redirect::Policy::default().redirect(attempt)
        }
    });
    reqwest::Client::builder()
        .dns_resolver(Arc::new(FilteredDnsResolver))
        .redirect(redirect_policy)
}

fn client_builder() -> reqwest::ClientBuilder {
    reqwest::Client::builder()
        .https_only(true)
        .redirect(reqwest::redirect::Policy::none())
        .no_proxy()
        .referer(false)
        .connection_verbose(false)
        .dns_resolver(Arc::new(FilteredDnsResolver))
}

struct StrictRequest {
    inner: reqwest::Request,
    response_limit: u32,
}

impl std::fmt::Debug for StrictRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("StrictRequest([REDACTED])")
    }
}

fn prepare_request(frame: RequestWithPolicy, body: Bytes) -> Result<StrictRequest, HttpPolicyError> {
    let response_limit = frame.policy.response_limit()?;
    let (mut parts, timeout) = convert_http_request(frame.request).map_err(|_| HttpPolicyError::InvalidRequest)?;
    // An explicit Host override would separate HTTP authority from the validated TLS origin.
    if parts.headers.contains_key(http::header::HOST) {
        return Err(HttpPolicyError::InvalidRequest);
    }
    for value in parts.headers.values_mut() {
        value.set_sensitive(true);
    }
    let request = http::Request::from_parts(parts, body);
    let mut request: reqwest::Request = request.try_into().map_err(|_| HttpPolicyError::InvalidRequest)?;
    if request.url().scheme() != "https" {
        return Err(HttpPolicyError::HttpsRequired);
    }
    if !request.url().username().is_empty() || request.url().password().is_some() {
        return Err(HttpPolicyError::InvalidRequest);
    }
    if is_blocked_ip_literal(request.url()) {
        return Err(HttpPolicyError::BlockedAddress);
    }
    *request.timeout_mut() = Some(timeout.unwrap_or(HTTP_DEFAULT_TIMEOUT).min(HTTP_MAX_TIMEOUT));
    Ok(StrictRequest {
        inner: request,
        response_limit,
    })
}

fn transport_error(error: reqwest::Error) -> HttpPolicyError {
    if error.is_timeout() {
        HttpPolicyError::Timeout
    } else {
        HttpPolicyError::Network
    }
}

async fn execute(frame: RequestWithPolicy, body: Bytes) -> Result<(st_http::Response, Bytes), HttpPolicyError> {
    let request = prepare_request(frame, body)?;
    let client = client_builder().build().map_err(|_| HttpPolicyError::HostFailure)?;
    execute_prepared(client, request).await
}

async fn execute_prepared(
    client: reqwest::Client,
    request: StrictRequest,
) -> Result<(st_http::Response, Bytes), HttpPolicyError> {
    let response = client.execute(request.inner).await.map_err(transport_error)?;
    if response
        .content_length()
        .is_some_and(|len| len > u64::from(request.response_limit))
    {
        return Err(HttpPolicyError::ResponseTooLarge);
    }
    let (parts, body) = http::Response::from(response).into_parts();
    let body = collect_limited(body.into_data_stream().map_err(transport_error), request.response_limit).await?;
    Ok((convert_http_response(parts), body))
}

async fn collect_limited(
    stream: impl Stream<Item = Result<Bytes, HttpPolicyError>>,
    limit: u32,
) -> Result<Bytes, HttpPolicyError> {
    futures_util::pin_mut!(stream);
    let mut body = Vec::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        if chunk.len() > (limit as usize).saturating_sub(body.len()) {
            return Err(HttpPolicyError::ResponseTooLarge);
        }
        body.try_reserve_exact(chunk.len())
            .map_err(|_| HttpPolicyError::HostFailure)?;
        body.extend_from_slice(&chunk);
    }
    Ok(body.into())
}

#[cfg(test)]
mod tests;
