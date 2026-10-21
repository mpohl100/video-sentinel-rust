#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use async_graphql::{Context, Object, Result};

use super::{
    BasicParamsGql, EyeSessionGql, ObjectSessionGql, OrdinarySessionGql, TileParamsGql,
    TraceParamsGql, app_state,
};

pub struct QueryRoot;

#[Object]
impl QueryRoot {
    async fn get_ordinary_session(
        &self,
        ctx: &Context<'_>,
        session_id: String,
    ) -> Result<Option<OrdinarySessionGql>> {
        let service = app_state(ctx)?.service;
        let service = service.lock().await;
        Ok(service.get_ordinary_session(&session_id).map(Into::into))
    }

    async fn get_eye_session(
        &self,
        ctx: &Context<'_>,
        session_id: String,
    ) -> Result<Option<EyeSessionGql>> {
        let service = app_state(ctx)?.service;
        let service = service.lock().await;
        Ok(service.get_eye_session(&session_id).map(Into::into))
    }

    async fn get_object_session(
        &self,
        ctx: &Context<'_>,
        session_id: String,
    ) -> Result<Option<ObjectSessionGql>> {
        let service = app_state(ctx)?.service;
        let service = service.lock().await;
        Ok(service.get_object_session(&session_id).map(Into::into))
    }

    async fn get_basic_params(
        &self,
        ctx: &Context<'_>,
        session_id: String,
    ) -> Result<Option<BasicParamsGql>> {
        let service = app_state(ctx)?.service;
        let service = service.lock().await;
        Ok(service.get_basic_params(&session_id).map(Into::into))
    }

    async fn get_tile_params(
        &self,
        ctx: &Context<'_>,
        session_id: String,
    ) -> Result<Option<TileParamsGql>> {
        let service = app_state(ctx)?.service;
        let service = service.lock().await;
        Ok(service.get_tile_params(&session_id).map(Into::into))
    }

    async fn get_trace_params(
        &self,
        ctx: &Context<'_>,
        session_id: String,
    ) -> Result<Option<TraceParamsGql>> {
        let service = app_state(ctx)?.service;
        let service = service.lock().await;
        Ok(service.get_trace_params(&session_id).map(Into::into))
    }

    async fn get_bucket_delta(&self, ctx: &Context<'_>, session_id: String) -> Result<Option<f64>> {
        let service = app_state(ctx)?.service;
        let service = service.lock().await;
        Ok(service.get_bucket_delta(&session_id))
    }

    async fn get_target_similarity(
        &self,
        ctx: &Context<'_>,
        session_id: String,
    ) -> Result<Option<f64>> {
        let service = app_state(ctx)?.service;
        let service = service.lock().await;
        Ok(service.get_target_similarity(&session_id))
    }
}
