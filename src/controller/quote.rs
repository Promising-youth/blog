use crate::common::{CommonResp, Resp};
use crate::model::quote::{self, Quote};
use actix_web::{get, web};

// 先上传图片，之后再上传media
#[allow(dead_code)]
pub async fn save_quote(quote: web::Json<Quote>) -> CommonResp {
    let quote = quote.into_inner();
    Resp::ok(quote::save_quote(quote)?).to_json()
}

#[get("/quote/random")]
pub async fn random_quote() -> CommonResp {
    Resp::ok(quote::get_random_quote()?).to_json()
}
