use super::*;
use spacetimedb_lib::http::{policy::decode_request, HttpPolicyError};

impl WasmInstanceEnv {
    /// ABI 10.6. Frame/transport failures return only a BSATN `HttpPolicyError`.
    /// Linear-memory pointer rules and success outputs match procedure_http_request.
    pub fn procedure_http_request_with_policy<'caller>(
        caller: Caller<'caller, Self>,
        (request_ptr, request_len, body_ptr, body_len, out): (WasmPtr<u8>, u32, WasmPtr<u8>, u32, WasmPtr<u32>),
    ) -> Fut<'caller, RtResult<u32>> {
        Self::async_with_span(
            caller,
            AbiCall::ProcedureHttpRequestWithPolicy,
            move |mut caller| async move {
                let (mem, env) = Self::mem_env(&mut caller);
                let res: WasmResult<u32> = async {
                    let request_buf = mem.deref_slice(request_ptr, request_len)?;
                    let result = match decode_request(request_buf) {
                        Ok(request) => {
                            let body = bytes::Bytes::copy_from_slice(mem.deref_slice(body_ptr, body_len)?);
                            env.instance_env.http_request_with_policy(request, body).await
                        }
                        Err(error) => Err(error),
                    };
                    match result {
                        Ok((response, body)) => {
                            let response = bsatn::to_vec(&response)
                                .map_err(|_| anyhow!("failed to encode strict HTTP response"))?;
                            let source = Self::create_bytes_source(env, response.into())?;
                            source.0.write_to(mem, out)?;
                            let source = Self::create_bytes_source(env, body)?;
                            source.0.write_to(mem, out.saturating_add(size_of::<u32>() as u32))?;
                            Ok(0u32)
                        }
                        Err(error) => {
                            write_policy_error(mem, env, out, error)?;
                            Ok(errno::HTTP_ERROR.get() as u32)
                        }
                    }
                }
                .await;
                (
                    caller,
                    res.or_else(|error| Self::convert_wasm_result(AbiCall::ProcedureHttpRequestWithPolicy, error)),
                )
            },
        )
    }
}

fn write_policy_error(
    mem: &mut MemView,
    env: &mut WasmInstanceEnv,
    out: WasmPtr<u32>,
    error: HttpPolicyError,
) -> RtResult<()> {
    let encoded = bsatn::to_vec(&error).map_err(|_| anyhow!("failed to encode strict HTTP error"))?;
    let source = WasmInstanceEnv::create_bytes_source(env, encoded.into())?;
    source.0.write_to(mem, out)?;
    Ok(())
}
