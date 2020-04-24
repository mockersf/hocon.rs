pub(crate) fn extract_result<T, E>(source: Vec<Result<T, E>>) -> Result<Vec<T>, E>
where
    E: std::fmt::Debug + Clone,
    T: Clone,
{
    if let Some(err) = source
        .iter()
        .filter(|v| v.is_err())
        .map(|v| {
            v.clone()
                .err()
                .expect("should be err as it was fitlered before")
        })
        .next()
    {
        return Err(err);
    }
    Ok(source
        .into_iter()
        .map(|v| v.expect("extract_result: got a Err"))
        .collect())
}
