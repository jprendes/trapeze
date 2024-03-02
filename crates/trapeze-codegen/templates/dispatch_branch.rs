"__method_proto_name__" => {
    let payload = trapeze::encoded::Encoded::decode(&payload)?;
    let response = super::__service_name__::__method_name__(&self.0, payload).await?;
    Ok(trapeze::encoded::Encoded::encode(&response)?)
},