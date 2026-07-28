use sqlx::PgPool;

#[derive(Clone, Debug)]
pub struct ApHelper {
    db_pool: PgPool,
    domain_name: String,
}

impl ApHelper {
    pub fn new(db_pool: PgPool, domain_name: String) -> Self {
        Self {
            db_pool,
            domain_name,
        }
    }

    pub fn get_db_pool(&self) -> &PgPool {
        &self.db_pool
    }

    pub fn get_domain_name(&self) -> &str {
        &self.domain_name
    }
}