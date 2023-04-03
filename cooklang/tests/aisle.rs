//! Shopping list configuration file test

use cooklang::aisle::*;

const CONF: &str = r#"
[produce]
potatoes

[dairy]
milk
butter

[deli]
chicken

[canned goods]
tuna|chicken of the sea

[empty category]

[another]
"#;

#[test]
fn test_shopping_list() {
    let got = parse(CONF).unwrap();

    let expected = AileConf {
        categories: vec![
            Category {
                name: "produce",
                ingredients: vec![Ingredient {
                    names: vec!["potatoes"],
                }],
            },
            Category {
                name: "dairy",
                ingredients: vec![
                    Ingredient {
                        names: vec!["milk"],
                    },
                    Ingredient {
                        names: vec!["butter"],
                    },
                ],
            },
            Category {
                name: "deli",
                ingredients: vec![Ingredient {
                    names: vec!["chicken"],
                }],
            },
            Category {
                name: "canned goods",
                ingredients: vec![Ingredient {
                    names: vec!["tuna", "chicken of the sea"],
                }],
            },
            Category {
                name: "empty category",
                ingredients: vec![],
            },
            Category {
                name: "another",
                ingredients: vec![],
            },
        ],
    };

    assert_eq!(expected, got);
}
