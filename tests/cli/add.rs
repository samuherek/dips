use crate::helpers::TestApp;
use dips::commands::add;
use dips::models::dip;
use fake::faker::lorem::en::Word;
use fake::Fake;

#[tokio::test]
async fn adding_new_value_should_be_added() {
    let setup = TestApp::setup().await;
    let application = setup.application();
    let input = Word().fake();
    add::add(application, input, None, false).await;

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
    add::add(application, input, None, false).await;

    let rows = dip::get_all(&application.db_pool).await.unwrap();
    assert_eq!(rows.len(), 1);
    let value = &rows[0];

    assert_eq!(value.value, input);
}

#[tokio::test]
async fn adding_new_value_with_group_stores_group() {
    let setup = TestApp::setup().await;
    let application = setup.application();
    let input = Word().fake();
    let group: String = Word().fake();
    add::add(application, input, Some(&group), false).await;

    let rows = dip::get_all(&application.db_pool).await.unwrap();
    assert_eq!(rows.len(), 1);
    let value = &rows[0];

    assert_eq!(value.value, input);
    assert!(value.dir_context_id.is_some());
    assert_eq!(value.context_group_name, Some(group));
}

#[tokio::test]
async fn adding_value_with_global_flag_stores_global() {
    let setup = TestApp::setup().await;
    let application = setup.application();
    let input = Word().fake();
    add::add(application, input, None, true).await;

    let rows = dip::get_all(&application.db_pool).await.unwrap();
    assert_eq!(rows.len(), 1);
    let value = &rows[0];

    assert_eq!(value.value, input);
    assert_eq!(value.dir_context_id, None);
    assert!(value.context_group_name.is_none());
}

#[tokio::test]
async fn add_existing_dir_context_value_complains() {
    let setup = TestApp::setup().await;
    let application = setup.application();
    let input = Word().fake();

    add::add(application, input, None, false).await;
    add::add(application, input, None, false).await;

    let rows = dip::get_all(&application.db_pool).await.unwrap();
    assert_eq!(rows.len(), 1);
}

#[tokio::test]
async fn add_existing_dir_context_and_group_value_complains() {
    let setup = TestApp::setup().await;
    let application = setup.application();
    let input = Word().fake();
    let group: String = Word().fake();

    add::add(application, input, Some(&group), false).await;
    add::add(application, input, Some(&group), false).await;

    let rows = dip::get_all(&application.db_pool).await.unwrap();
    assert_eq!(rows.len(), 1);
}

#[tokio::test]
async fn add_existing_global_value_complains() {
    let setup = TestApp::setup().await;
    let application = setup.application();
    let input = Word().fake();

    add::add(application, input, None, true).await;
    add::add(application, input, None, true).await;

    let rows = dip::get_all(&application.db_pool).await.unwrap();
    assert_eq!(rows.len(), 1);
}

#[tokio::test]
async fn add_existing_global_group_value_complains() {
    let setup = TestApp::setup().await;
    let application = setup.application();
    let input = Word().fake();
    let group: String = Word().fake();

    add::add(application, input, Some(&group), true).await;
    add::add(application, input, Some(&group), true).await;

    let rows = dip::get_all(&application.db_pool).await.unwrap();
    assert_eq!(rows.len(), 1);
}
