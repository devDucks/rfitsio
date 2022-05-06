.PHONY = build-doc

build-doc:
	@cargo doc

.PHONY = doc

doc:
	@cargo doc --open
