#/bin/bash

#variable 
IMAGE_TAG="ubuntu-grpc:v0.1"


script_path=$(readlink -f "${BASH_SOURCE:-$0}")
script_dir=$(dirname $script_path)
echo $script_dir

working_dir=$script_dir/../
cd $working_dir
working_dir=$(pwd -P)
cd -


proto_path=${script_dir}/../../../Protos/V1/

cd $proto_path
proto_dir=$(pwd -P)
cd -

mkdir -p ${working_dir}/build
cd ${working_dir}/build
build_dir=$(pwd -P)
cd -

mkdir -p ${working_dir}/install
cd ${working_dir}/install
install_dir=$(pwd -P)
cd -

echo "Working dir          : ${working_dir}"
echo "Directory of proto   : ${proto_dir}"
echo "Directory of build   : ${build_dir}"
echo "Directory of install : ${install_dir}"

cd ${working_dir}

if [[ "$(docker images -q ${IMAGE_TAG} 2> /dev/null)" == "" ]]; then
  echo "Build docker image ${IMAGE_TAG} to compile Armonik.Api.Cpp"
  docker build -t ${IMAGE_TAG} -f tools/Dockerfile.ubuntu .
fi

#docker build -t ${IMAGE_TAG} -f tools/Dockerfile.ubuntu .
# mkdir -p ${working_dir}/Protos/
# cp -r $script_dir/../../../Protos/V1/* ${working_dir}/Protos/
echo "Compiling project source"
docker run -v ${proto_dir}:/app/proto -v ${working_dir}:/app/source -v ${build_dir}:/app/build -v ${install_dir}:/app/install --rm -it ${IMAGE_TAG}
#docker run -v ${proto_dir}:/app/proto -v ${working_dir}:/app/source -v ${build_dir}:/app/build -v ${install_dir}:/app/install --rm -it ${IMAGE_TAG} bash