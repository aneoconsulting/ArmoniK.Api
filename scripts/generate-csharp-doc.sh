#!/bin/sh

set -e
dotnet tool update -g docfx
dotnet build packages/csharp/ArmoniK.Api.sln
docfx docfx.json
sed -E -i 's/([#]+) <a id="([^"]+)"><\/a> (.+)/<a id="\2"><\/a>\n\1 \3/g' .docs/content/api/csharp/*.md