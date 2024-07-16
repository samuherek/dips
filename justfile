

release-dry:
    cargo release patch --no-publish

release-patch:
    cargo release patch --no-publish --execute
