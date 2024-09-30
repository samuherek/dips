use crate::helpers::TestApp;
use dips::commands::add;
use dips::models::dip::db_all;
use fake::faker::lorem::en::Word;
use fake::Fake;

#[tokio::test]
async fn adding_new_value_should_be_added() {
    let setup = TestApp::setup().await;
    let application = setup.application();
    let input = Word().fake();
    add::add(application, input, &None).await;

    let value = &db_all(&application.db_pool).await.unwrap()[0];

    assert_eq!(value.value, input);
}
