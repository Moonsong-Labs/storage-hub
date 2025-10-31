use bigdecimal::BigDecimal;
use chrono::NaiveDateTime;
use diesel::{dsl::sum, prelude::*};
use diesel_async::RunQueryDsl;

use crate::{
    schema::{bucket, file},
    DbConnection,
};

/// Table that holds the Buckets.
#[derive(Debug, Clone, Queryable, Insertable, Selectable)]
#[diesel(table_name = bucket)]
#[diesel(belongs_to(Msp, foreign_key = msp_id))]
pub struct Bucket {
    /// The ID of the Bucket as stored in the database. For the runtime id, use `onchain_bucket_id`.
    pub id: i64,
    /// The ID of the MSP (column in the database) that the bucket belongs to.
    pub msp_id: Option<i64>,
    pub account: String,
    /// The onchain Bucket ID
    ///
    /// Generally, the bucket ID is a H256
    pub onchain_bucket_id: Vec<u8>,
    pub name: Vec<u8>,
    pub collection_id: Option<String>,
    pub private: bool,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub merkle_root: Vec<u8>,
    pub value_prop_id: String,
    pub total_size: BigDecimal,
    pub file_count: i64,
}

impl Bucket {
    pub async fn create<'a>(
        conn: &mut DbConnection<'a>,
        msp_id: Option<i64>,
        account: String,
        onchain_bucket_id: Vec<u8>,
        name: Vec<u8>,
        collection_id: Option<String>,
        private: bool,
        merkle_root: Vec<u8>,
        value_prop_id: String,
    ) -> Result<Self, diesel::result::Error> {
        let bucket = diesel::insert_into(bucket::table)
            .values((
                bucket::msp_id.eq(msp_id),
                bucket::account.eq(account),
                bucket::onchain_bucket_id.eq(onchain_bucket_id),
                bucket::name.eq(name),
                bucket::collection_id.eq(collection_id),
                bucket::private.eq(private),
                bucket::merkle_root.eq(merkle_root),
                bucket::value_prop_id.eq(value_prop_id),
                bucket::total_size.eq(BigDecimal::from(0)),
                bucket::file_count.eq(0i64),
            ))
            .returning(Bucket::as_select())
            .get_result(conn)
            .await?;
        Ok(bucket)
    }

    pub async fn update_privacy<'a>(
        conn: &mut DbConnection<'a>,
        account: String,
        onchain_bucket_id: Vec<u8>,
        collection_id: Option<String>,
        private: bool,
    ) -> Result<Self, diesel::result::Error> {
        let bucket = diesel::update(bucket::table)
            .filter(bucket::account.eq(account))
            .filter(bucket::onchain_bucket_id.eq(onchain_bucket_id))
            .set((
                bucket::collection_id.eq(collection_id),
                bucket::private.eq(private),
            ))
            .returning(Bucket::as_select())
            .get_result(conn)
            .await?;
        Ok(bucket)
    }

    pub async fn update_msp<'a>(
        conn: &mut DbConnection<'a>,
        onchain_bucket_id: Vec<u8>,
        msp_id: i64,
    ) -> Result<Self, diesel::result::Error> {
        let bucket = diesel::update(bucket::table)
            .filter(bucket::onchain_bucket_id.eq(onchain_bucket_id))
            .set(bucket::msp_id.eq(msp_id))
            .returning(Bucket::as_select())
            .get_result(conn)
            .await?;
        Ok(bucket)
    }

    pub async fn unset_msp<'a>(
        conn: &mut DbConnection<'a>,
        onchain_bucket_id: Vec<u8>,
    ) -> Result<(), diesel::result::Error> {
        diesel::update(bucket::table)
            .filter(bucket::onchain_bucket_id.eq(onchain_bucket_id))
            .set(bucket::msp_id.eq(None::<i64>))
            .execute(conn)
            .await?;
        Ok(())
    }

    pub async fn update_merkle_root<'a>(
        conn: &mut DbConnection<'a>,
        onchain_bucket_id: Vec<u8>,
        merkle_root: Vec<u8>,
    ) -> Result<(), diesel::result::Error> {
        diesel::update(bucket::table)
            .filter(bucket::onchain_bucket_id.eq(onchain_bucket_id))
            .set(bucket::merkle_root.eq(merkle_root))
            .execute(conn)
            .await?;
        Ok(())
    }

    pub async fn delete<'a>(
        conn: &mut DbConnection<'a>,
        onchain_bucket_id: Vec<u8>,
    ) -> Result<(), diesel::result::Error> {
        diesel::delete(bucket::table)
            .filter(bucket::onchain_bucket_id.eq(onchain_bucket_id))
            .execute(conn)
            .await?;
        Ok(())
    }

    pub async fn get_by_id<'a>(
        conn: &mut DbConnection<'a>,
        id: i64,
    ) -> Result<Self, diesel::result::Error> {
        let bucket = bucket::table.filter(bucket::id.eq(id)).first(conn).await?;
        Ok(bucket)
    }

    pub async fn get_by_onchain_bucket_id<'a>(
        conn: &mut DbConnection<'a>,
        onchain_bucket_id: Vec<u8>,
    ) -> Result<Self, diesel::result::Error> {
        let bucket = bucket::table
            .filter(bucket::onchain_bucket_id.eq(onchain_bucket_id))
            .first(conn)
            .await?;
        Ok(bucket)
    }

    /// Get all buckets belonging to a specific user account
    pub async fn get_user_buckets<'a>(
        conn: &mut DbConnection<'a>,
        user_account: impl Into<String>,
    ) -> Result<Vec<Self>, diesel::result::Error> {
        let account = user_account.into();
        let buckets = bucket::table
            .filter(bucket::account.eq(account))
            .load(conn)
            .await?;
        Ok(buckets)
    }

    /// Get all buckets belonging to a specific user account and assigned to a specific MSP
    pub async fn get_user_buckets_by_msp<'a>(
        conn: &mut DbConnection<'a>,
        user_account: impl Into<String>,
        msp_id: i64,
    ) -> Result<Vec<Self>, diesel::result::Error> {
        let account = user_account.into();
        let buckets = bucket::table
            .filter(bucket::account.eq(account))
            .filter(bucket::msp_id.eq(msp_id))
            .load(conn)
            .await?;
        Ok(buckets)
    }

    /// Calculate the total size of all files in a bucket
    pub async fn calculate_size<'a>(
        conn: &mut DbConnection<'a>,
        bucket_id: i64,
    ) -> Result<BigDecimal, diesel::result::Error> {
        let total_size: Option<BigDecimal> = file::table
            .filter(file::bucket_id.eq(bucket_id))
            .select(sum(file::size))
            .first(conn)
            .await?;

        // Return BigDecimal directly, defaulting to zero if None
        Ok(total_size.unwrap_or_else(|| BigDecimal::from(0)))
    }

    /// Update the total_size and file_count for a bucket
    pub async fn update_size_and_count<'a>(
        conn: &mut DbConnection<'a>,
        bucket_id: i64,
        total_size: BigDecimal,
        file_count: i64,
    ) -> Result<(), diesel::result::Error> {
        diesel::update(bucket::table)
            .filter(bucket::id.eq(bucket_id))
            .set((
                bucket::total_size.eq(total_size),
                bucket::file_count.eq(file_count),
            ))
            .execute(conn)
            .await?;
        Ok(())
    }

    /// Increment file count and update total size
    pub async fn increment_file_count_and_size<'a>(
        conn: &mut DbConnection<'a>,
        bucket_id: i64,
        file_size: i64,
    ) -> Result<(), diesel::result::Error> {
        let size_decimal = BigDecimal::from(file_size);
        diesel::update(bucket::table)
            .filter(bucket::id.eq(bucket_id))
            .set((
                bucket::total_size.eq(bucket::total_size + size_decimal),
                bucket::file_count.eq(bucket::file_count + 1),
            ))
            .execute(conn)
            .await?;
        Ok(())
    }

    /// Decrement file count and update total size
    pub async fn decrement_file_count_and_size<'a>(
        conn: &mut DbConnection<'a>,
        bucket_id: i64,
        file_size: i64,
    ) -> Result<(), diesel::result::Error> {
        let size_decimal = BigDecimal::from(file_size);
        diesel::update(bucket::table)
            .filter(bucket::id.eq(bucket_id))
            .set((
                bucket::total_size.eq(bucket::total_size - size_decimal),
                bucket::file_count.eq(bucket::file_count - 1),
            ))
            .execute(conn)
            .await?;
        Ok(())
    }
}
