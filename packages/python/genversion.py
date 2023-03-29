from setuptools_scm.version import ScmVersion, guess_next_version
from setuptools_scm import Configuration, _get_version
from argparse import ArgumentParser
from hashlib import sha256


class Versionner:
    def __init__(self, consider_dirty=True):
        self.consider_dirty = consider_dirty

    def get_dev_version(self, version: ScmVersion):
        branch_code = f"b{int(sha256(version.branch.encode('ascii')).hexdigest()[:7], 16)}" if version.branch != "main" else ""
        if version.dirty and self.consider_dirty:
            return version.format_next_version(guess_next_version,
                                               "{guessed}" + f"{branch_code}.post{int(version.node[1:], 16)}.dev0")
        if version.distance == 0:
            return self.get_rel_version(version)
        return version.format_next_version(guess_next_version,
                                           "{guessed}" + f"{branch_code}.post{int(version.node[1:], 16)}")

    def get_rel_version(self, version: ScmVersion):
        if version.dirty and self.consider_dirty:
            return version.format_next_version(guess_next_version, "{guessed}.dev0")
        if version.distance == 0:
            return version.format_with("{tag}")
        return version.format_next_version(guess_next_version, "{guessed}")


def main():
    parser = ArgumentParser("ArmoniK Python version generator")
    parser.add_argument("-r", "--release", action='store_true')
    parser.add_argument("-n", "--no-dirty", action='store_true')
    args = parser.parse_args()
    config = Configuration.from_file("./pyproject.toml")
    versionner = Versionner(not args.no_dirty)
    config.version_scheme = versionner.get_rel_version if args.release else versionner.get_dev_version
    version = _get_version(config)
    print(version)


if __name__ == "__main__":
    main()
