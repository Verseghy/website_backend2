.PHONY: deploy
deploy:
	docker build . -t verseghy/website_backend2
	docker push verseghy/website_backend2
	kubectl rollout restart deployment backend -n testing
