from setuptools_scm.version import ScmVersion, guess_next_version
from setuptools_scm import Configuration, _get_version
from argparse import ArgumentParser
from hashlib import sha256


def get_dev_version(version: ScmVersion):
    return version.format_next_version(guess_next_version, "{guessed}b"+f"{int(sha256(version.branch.encode('ascii')).hexdigest()[:7], 16)}.dev{int(version.node[1:], 16)}")


def get_rel_version(version: ScmVersion):
    return version.format_next_version(guess_next_version, "{guessed}")


def main():
    parser = ArgumentParser("ArmoniK Python version generator")
    parser.add_argument("-r", "--release", action='store_true')
    args = parser.parse_args()
    config = Configuration.from_file("./pyproject.toml")
    config.version_scheme = get_rel_version if args.release else get_dev_version
    version = _get_version(config)
    print(version)


if __name__ == "__main__":
    main()
