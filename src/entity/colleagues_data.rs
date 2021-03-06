//! SeaORM Entity. Generated by sea-orm-codegen 0.4.2

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "colleagues_data")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub name: Option<String>,
    #[sea_orm(column_type = "Custom(\"LONGTEXT\".to_owned())", nullable)]
    pub jobs: Option<String>,
    #[sea_orm(column_type = "Custom(\"LONGTEXT\".to_owned())", nullable)]
    pub subjects: Option<String>,
    #[sea_orm(column_type = "Custom(\"LONGTEXT\".to_owned())", nullable)]
    pub roles: Option<String>,
    #[sea_orm(column_type = "Custom(\"LONGTEXT\".to_owned())", nullable)]
    pub awards: Option<String>,
    pub image: Option<String>,
    pub category: Option<i16>,
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        panic!("No RelationDef")
    }
}

impl ActiveModelBehavior for ActiveModel {}
