use crate::helpers::setup_app;

#[tokio::test]
async fn try_test() {
    let app = setup_app().await;
    println!("app {:?}", app);
}
