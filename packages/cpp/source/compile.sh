#/bin/bash

script_path=$(readlink -f "${BASH_SOURCE:-$0}")
script_dir=$(dirname $script_path)
echo $script_dir

working_dir=$(pwd -P)

proto_path=${script_dir}/../../Protos/V1/

cd $proto_path
proto_dir=$(pwd -P)
cd -

mkdir -p ${working_dir}/build
cd ${working_dir}/build
build_dir=$(pwd -P)
cd -

mkdir -p ${working_dir}/../install
cd ${working_dir}/../install
install_dir=$(pwd -P)
cd -

echo "Working dir          : ${working_dir}"
echo "Directory of proto   : ${proto_dir}"
echo "Directory of build   : ${build_dir}"
echo "Directory of install : ${install_dir}"

cd ${working_dir}

docker -D run -v ${proto_dir}:/app/proto -v ${working_dir}:/app -v ${build_dir}:/app/build -v ${install_dir}:/app/install --rm -it ubuntu-grpc
