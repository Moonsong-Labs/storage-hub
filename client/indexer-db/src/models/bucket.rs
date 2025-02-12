use chrono::NaiveDateTime;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;

use crate::{schema::bucket, DbConnection};

/// Table that holds the Buckets.
#[derive(Debug, Queryable, Insertable, Selectable)]
#[diesel(table_name = bucket)]
#[diesel(belongs_to(Msp, foreign_key = msp_id))]
pub struct Bucket {
    /// The ID of the Bucket as stored in the database. For the runtime id, use `onchain_bucket_id`.
    pub id: i64,
    /// The ID of the MSP (column in the database) that the bucket belongs to.
    pub msp_id: Option<i64>,
    pub account: String,
    pub onchain_bucket_id: Vec<u8>,
    pub name: Vec<u8>,
    pub collection_id: Option<String>,
    pub private: bool,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub merkle_root: Vec<u8>,
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

    pub async fn get_by_owner<'a>(
        conn: &mut DbConnection<'a>,
        account: String,
    ) -> Result<Vec<Self>, diesel::result::Error> {
        let buckets = bucket::table
            .filter(bucket::account.eq(account))
            .load(conn)
            .await?;
        Ok(buckets)
    }
}
