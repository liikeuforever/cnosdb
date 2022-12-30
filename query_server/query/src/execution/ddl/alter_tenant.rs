use crate::execution::ddl::DDLDefinitionTask;
use async_trait::async_trait;
use meta::error::MetaError;
use snafu::ResultExt;
use spi::query::execution::{ExecutionError, MetadataSnafu, Output, QueryStateMachineRef};
use spi::query::logical_planner::{
    AlterTenant, AlterTenantAction, AlterTenantAddUser, AlterTenantSetUser,
};
use trace::debug;

pub struct AlterTenantTask {
    stmt: AlterTenant,
}

impl AlterTenantTask {
    pub fn new(stmt: AlterTenant) -> AlterTenantTask {
        Self { stmt }
    }
}

#[async_trait]
impl DDLDefinitionTask for AlterTenantTask {
    async fn execute(
        &self,
        query_state_machine: QueryStateMachineRef,
    ) -> Result<Output, ExecutionError> {
        let AlterTenant {
            ref tenant_name,
            ref alter_tenant_action,
        } = self.stmt;

        let tenant_manager = query_state_machine.meta.tenant_manager();

        let meta =
            tenant_manager
                .tenant_meta(tenant_name)
                .ok_or_else(|| ExecutionError::Metadata {
                    source: MetaError::TenantNotFound {
                        tenant: tenant_name.to_string(),
                    },
                })?;

        match alter_tenant_action {
            AlterTenantAction::AddUser(AlterTenantAddUser { user_id, role }) => {
                // 向租户中添加指定角色的成员
                // user_id: Oid,
                // role: TenantRoleIdentifier,
                // tenant_id: Oid,
                // fn add_member_with_role_to_tenant(
                //     &mut self,
                //     user_id: Oid,
                //     role: TenantRoleIdentifier,
                //     tenant_id: Oid,
                // ) -> Result<()>
                debug!(
                    "Add user {} to tenant {} with role {:?}",
                    user_id, tenant_name, role
                );
                meta.add_member_with_role(*user_id, role.clone())
                    .context(MetadataSnafu)?;
            }
            AlterTenantAction::SetUser(AlterTenantSetUser { user_id, role }) => {
                // 重设租户中指定成员的角色
                // user_id: Oid,
                // role: TenantRoleIdentifier,
                // tenant_id: Oid,
                // fn reasign_member_role_in_tenant(
                //     &mut self,
                //     user_id: Oid,
                //     role: TenantRoleIdentifier,
                //     tenant_id: Oid,
                // ) -> Result<()>;
                debug!(
                    "Reasign role {:?} of user {} in tenant {}",
                    role, user_id, tenant_name
                );
                meta.reasign_member_role(*user_id, role.clone())
                    .context(MetadataSnafu)?;
            }
            AlterTenantAction::RemoveUser(user_id) => {
                // 从租户中移除指定成员
                // user_id: Oid,
                // tenant_id: Oid
                // fn remove_member_from_tenant(
                //     &mut self,
                //     user_id: Oid,
                //     tenant_id: Oid
                // ) -> Result<()>;
                debug!("Remove user {} from tenant {}", user_id, tenant_name);
                meta.remove_member(*user_id).context(MetadataSnafu)?;
            }
            AlterTenantAction::Set(options) => {
                // TODO 修改租户的信息
                // tenant_id: Oid,
                // options: TenantOptions
                // fn alter_tenant(
                //     &self,
                //     tenant_id: Oid,
                //     options: TenantOptions
                // ) -> Result<()>;
                debug!("Alter tenant {} with options [{}]", tenant_name, options);
                tenant_manager
                    .alter_tenant(tenant_name, *options.clone())
                    .context(MetadataSnafu)?;
            }
        }

        return Ok(Output::Nil(()));
    }
}