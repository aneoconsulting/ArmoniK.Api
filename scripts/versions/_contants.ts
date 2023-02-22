import glob from "glob";

export const csharpPattern = /<PackageVersion>(?<version>.*)<\/PackageVersion>/;
export const csharpFiles = glob.sync(`**/*.csproj`);

export const pythonPattern = /version = "(?<version>.*)"/g;
export const pythonFiles = ["packages/python/pyproject.toml"];

export const jsPattern = /"version": "(?<version>.*)"/;
export const jsFiles = ["packages/angular/projects/aneoconsultingfr/armonik.api.angular/package.json", "packages/web/package.json"];
