pub struct InstanceInfo {
    /// MicroVM / instance ID.
    id: String,
    /// The current detailed state of the Firecracker instance. This value is read-only for the control-plane. = ['Uninitialized', 'Starting', 'Running']
    state: String,
    /// MicroVM hypervisor build version.
    vmm_version: String,
    /// Application name.
    app_name: Option<String>,
}

pub struct InstanceActionInfo {
    /// Enumeration indicating what type of action is contained in the payload = ['FlushMetrics', 'InstanceStart', 'SendCtrlAltDel']
    action_type: String,
}

// Returns general information about an instance.
pub fn get_instance_info() -> Result<InstanceInfo, String> {
    Ok(InstanceInfo {
        id: "none".to_owned(),
        state: "none".to_owned(),
        vmm_version: "none".to_owned(),
        app_name: Some("container".to_owned()),
    })
}

/// Creates a synchronous action.
pub fn create_action(info: InstanceActionInfo) -> Result<(), String> {
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
