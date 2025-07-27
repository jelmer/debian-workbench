update: update-key-package-versions

update-key-package-versions:
	python3 key-package-versions.py key-package-versions.json
	brz diff key-package-versions.json || brz commit -m "Update key package versions" key-package-versions.json

.PHONY: update-key-package-versions update
