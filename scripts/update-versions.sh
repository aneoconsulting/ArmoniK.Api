#! /bin/bash

set -e

APP=$0

new_version=$1

if [ -z "$new_version" ]
then
  echo "usage: $APP <new version>"
  echo "example: $APP \"3.21.0\""
  exit 1
fi


__update-csharp() {
  projects=$(find ./ -name "*.csproj")
  for prj in $projects
  do
    echo "updating $prj..."
    sed -i "s/<PackageVersion>.*</<PackageVersion>$new_version</" $prj
    sed -i "s/<Version>.*</<Version>$new_version</" $prj
  done
}


__update-csharp
