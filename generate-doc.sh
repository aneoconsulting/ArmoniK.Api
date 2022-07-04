docker run --rm \
	-v $(pwd)/Documentation/api:/out -v \
	$(pwd)/Protos/V1:/protos pseudomuto/protoc-gen-doc
