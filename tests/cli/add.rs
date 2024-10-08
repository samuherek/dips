use crate::helpers::TestApp;
use dips::commands::add;
use dips::models::dip::{self, db_all};
use fake::faker::lorem::en::Word;
use fake::Fake;

#[tokio::test]
async fn adding_new_value_should_be_added() {
    let setup = TestApp::setup().await;
    let application = setup.application();
    let input = Word().fake();
    add::add(application, input, &None).await;

    let rows = dip::get_all(&application.db_pool).await.unwrap();
    assert_eq!(rows.len(), 1);
    let value = &rows[0];

    assert_eq!(value.value, input);
}

#[tokio::test]
async fn adding_new_value_should_have_correct_dir_context() {
    let setup = TestApp::setup().await;
    let application = setup.application();
    let input = Word().fake();
    add::add(application, input, &None).await;

    let rows = dip::get_all(&application.db_pool).await.unwrap();
    assert_eq!(rows.len(), 1);
    let value = &rows[0];

    assert_eq!(value.value, input);
}
