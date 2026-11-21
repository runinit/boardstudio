//! Ordering guard for the PCB handoff export: protection commits only after packaging.
//! Generic over caller-supplied steps, so it runs natively.

use std::future::Future;

/// Keep the irreversible protection commit behind successful package creation
/// and the same captured-owner checks used by the real Runtime path.
pub async fn package_then_protect<
    T,
    Package,
    IsCurrentBeforeProtect,
    Protect,
    ProtectFuture,
    IsCurrentBeforeDelivery,
>(
    package: Package,
    is_current_before_protect: IsCurrentBeforeProtect,
    protect: Protect,
    is_current_before_delivery: IsCurrentBeforeDelivery,
) -> Result<(Vec<u8>, T), String>
where
    Package: Future<Output = Result<Vec<u8>, String>>,
    IsCurrentBeforeProtect: FnOnce() -> Result<(), String>,
    Protect: FnOnce() -> ProtectFuture,
    ProtectFuture: Future<Output = Result<T, String>>,
    IsCurrentBeforeDelivery: FnOnce(&T) -> Result<(), String>,
{
    let bytes = package.await?;
    is_current_before_protect()?;
    let protection = protect().await?;
    is_current_before_delivery(&protection)?;
    Ok((bytes, protection))
}

#[cfg(test)]
mod tests;
