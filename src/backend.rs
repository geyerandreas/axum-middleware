use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    fmt::{Debug, Display},
    hash::Hash,
};

pub type UserId<Backend> = <<Backend as AuthnBackend>::User as AuthUser>::Id;

pub trait AuthUser: Debug + Clone + Send + Sync {
    type Id: Debug + Display + Clone + Send + Sync + Serialize + for<'de> Deserialize<'de>;

    fn id(&self) -> Self::Id;

    fn session_auth_hash(&self) -> &[u8];
}

pub trait AuthnBackend: Debug + Clone + Send + Sync {
    type User: AuthUser;

    type Credentials: Send + Sync;

    type Error: std::error::Error + Send + Sync;

    fn authenticate(
        &self,
        creds: Self::Credentials,
    ) -> impl Future<Output = Result<Option<Self::User>, Self::Error>> + Send;

    fn get_user(
        &self,
        user_id: &UserId<Self>,
    ) -> impl Future<Output = Result<Option<Self::User>, Self::Error>> + Send;
}

pub trait AuthzBackend
where
    Self: AuthnBackend,
{
    type Permission: Hash + Eq + Send + Sync;

    fn get_user_permissions(
        &self,
        _user: &Self::User,
    ) -> impl Future<Output = Result<HashSet<Self::Permission>, Self::Error>> + Send {
        async { Ok(HashSet::new()) }
    }

    fn get_group_permissions(
        &self,
        _user: &Self::User,
    ) -> impl Future<Output = Result<HashSet<Self::Permission>, Self::Error>> + Send {
        async { Ok(HashSet::new()) }
    }

    fn get_all_permissions(
        &self,
        user: &Self::User,
    ) -> impl Future<Output = Result<HashSet<Self::Permission>, Self::Error>> + Send {
        async {
            let mut all_perms = HashSet::new();
            all_perms.extend(self.get_user_permissions(user).await?);
            all_perms.extend(self.get_group_permissions(user).await?);
            Ok(all_perms)
        }
    }

    fn has_perm(
        &self,
        user: &Self::User,
        perm: &Self::Permission,
    ) -> impl Future<Output = Result<bool, Self::Error>> + Send {
        async move { Ok(self.get_all_permissions(user).await?.contains(perm)) }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    struct TestUser {
        id: i64,
        pw_hash: Vec<u8>,
    }

    impl AuthUser for TestUser {
        type Id = i64;

        fn id(&self) -> Self::Id {
            self.id
        }

        fn session_auth_hash(&self) -> &[u8] {
            &self.pw_hash
        }
    }

    #[derive(Debug, Clone)]
    struct TestBackend {
        users: HashMap<i64, TestUser>,
        user_permissions: HashMap<i64, HashSet<String>>,

        groups: HashMap<String, HashSet<i64>>,
        group_permissions: HashMap<String, HashSet<String>>,
    }

    impl TestBackend {
        fn new() -> Self {
            TestBackend {
                users: HashMap::new(),
                user_permissions: HashMap::new(),
                groups: HashMap::new(),
                group_permissions: HashMap::new(),
            }
        }

        fn add_user(&mut self, user: TestUser, permissions: Vec<String>) {
            self.users.insert(user.id, user.clone());
            self.user_permissions
                .insert(user.id, permissions.into_iter().collect());
        }

        fn add_group(&mut self, group: String, permissions: Vec<String>) {
            self.groups.insert(group.clone(), HashSet::new());
            self.group_permissions
                .insert(group, permissions.into_iter().collect());
        }

        fn add_user_to_group(&mut self, user: TestUser, group: String) {
            self.groups.entry(group).and_modify(|members| {
                members.insert(user.id);
            });
        }
    }

    impl AuthnBackend for TestBackend {
        type User = TestUser;
        type Credentials = i64;
        type Error = std::convert::Infallible;

        async fn authenticate(
            &self,
            creds: Self::Credentials,
        ) -> Result<Option<Self::User>, Self::Error> {
            Ok(self.users.get(&creds).cloned())
        }

        async fn get_user(
            &self,
            user_id: &UserId<Self>,
        ) -> Result<Option<Self::User>, Self::Error> {
            Ok(self.users.get(user_id).cloned())
        }
    }

    impl AuthzBackend for TestBackend {
        type Permission = String;

        async fn get_user_permissions(
            &self,
            user: &Self::User,
        ) -> Result<HashSet<Self::Permission>, Self::Error> {
            Ok(self
                .user_permissions
                .get(&user.id)
                .cloned()
                .unwrap_or_default())
        }

        async fn get_group_permissions(
            &self,
            user: &Self::User,
        ) -> Result<HashSet<Self::Permission>, Self::Error> {
            let belongs_to = self
                .groups
                .iter()
                .filter_map(|(group, members)| {
                    if members.contains(&user.id) {
                        Some(group)
                    } else {
                        None
                    }
                })
                .collect::<HashSet<_>>();

            let group_permissions = self
                .group_permissions
                .iter()
                .filter_map(|(group, permissions)| {
                    if belongs_to.contains(group) {
                        Some(permissions)
                    } else {
                        None
                    }
                })
                .flatten()
                .cloned()
                .collect();

            Ok(group_permissions)
        }
    }

    #[tokio::test]
    async fn test_authenticate() {
        let user = TestUser {
            id: 1,
            pw_hash: vec![1, 2, 3],
        };
        let mut backend = TestBackend::new();
        backend.add_user(user.clone(), vec![]);

        let authenticated_user = backend.authenticate(1).await.unwrap();
        assert_eq!(authenticated_user, Some(user));
    }

    #[tokio::test]
    async fn test_authenticate_failure() {
        let backend = TestBackend::new();

        assert!(backend.authenticate(1).await.unwrap().is_none());
    }
}
