docker run --rm \
	-v $(pwd)/doc:/out -v \
	$(pwd)/Protos/V1:/protos pseudomuto/protoc-gen-doc \
	--doc_opt=markdown,protobuf-generated-documentation.md	
