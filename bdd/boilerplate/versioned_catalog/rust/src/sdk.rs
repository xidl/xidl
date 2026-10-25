mod first {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/selected/v1/versioned_catalog.rs"
    ));
}
mod current {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/selected/v2/versioned_catalog.rs"
    ));
}

pub async fn exercise(base: String) -> Result<(), Box<dyn std::error::Error>> {
    let first = first::v1::CatalogClient::new(base.clone());
    let current = current::v2::CatalogClient::new(base);
    let baseline = current.search(vec![], "shop-a".into()).await?;
    assert_eq!(baseline.products.len(), 4);
    assert_eq!(baseline.products[3].assets[0].kind, "video");

    for (filters, ids) in [
        (vec![], vec!["image", "clip", "model", "video-only"]),
        (vec!["video"], vec!["clip", "model", "video-only"]),
        (vec!["video", "model_3d"], vec!["model"]),
        (vec!["video", "video"], vec!["clip", "model", "video-only"]),
    ] {
        let filters: Vec<String> = filters.into_iter().map(str::to_owned).collect();
        let (old, head) = tokio::join!(
            first.search(filters.clone(), "shop-a".into()),
            current.search(filters, "shop-a".into()),
        );
        let old = old?;
        let head = head?;
        assert_eq!(
            old.products
                .iter()
                .map(|product| product.id.as_str())
                .collect::<Vec<_>>(),
            ids
        );
        assert_eq!(
            head.products
                .iter()
                .map(|product| product.id.as_str())
                .collect::<Vec<_>>(),
            ids
        );
        assert!(old.messages.is_empty());
        assert!(head.messages.is_empty());
        if let Some(product) = old
            .products
            .iter()
            .find(|product| product.id == "video-only")
        {
            assert!(
                product.images.is_empty(),
                "the video match survives image-only projection"
            );
        }
    }
    let unknown = first
        .search(
            vec!["video".into(), "hologram".into(), "hologram".into()],
            "shop-a".into(),
        )
        .await?;
    assert_eq!(unknown.products.len(), 3);
    assert_eq!(unknown.messages.len(), 1);
    assert_eq!(unknown.messages[0].code, "not_found");
    assert_eq!(unknown.messages[0].value, "hologram");
    let private = current
        .search(vec!["video".into()], "shop-b".into())
        .await?;
    assert_eq!(private.products.len(), 1);
    assert_eq!(private.products[0].id, "private");
    assert!(matches!(
        first.search(vec![], "unknown".into()).await,
        Err(first::v1::CatalogSearchError::Denied(_))
    ));
    assert_eq!(
        current.search(vec![], "shop-a".into()).await?,
        baseline,
        "historical projections never rewrite Head assets"
    );
    println!("PASS: independent Rust SDKs preserve Head catalog filtering");
    Ok(())
}
