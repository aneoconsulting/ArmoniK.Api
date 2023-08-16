Name:           libarmonik
Version:        3.10.0
Release:        1%{?dist}
Summary:        ArmoniK Api libraries

License:        Apache 2.0
URL:            https://github.com/aneoconsulting/ArmoniK.Api
Source0:        %{name}-%{version}.tar.gz

BuildRequires: cmake, rh-python38-python-devel, centos-release-scl, devtoolset-10

%description


%package        devel
Summary:        Development files for %{name}
Requires:       %{name}%{?_isa} = %{version}-%{release}

%description    devel
The %{name}-devel package contains libraries and header files for
developing applications that use %{name}.


%prep
%setup -q


%build
%cmake -DBUILD_WORKER=ON -DBUILD_CLIENT=ON -DBUILD_SHARED_LIBS=ON -DFETCHCONTENT_FULLY_DISCONNECTED=OFF -DPROTO_FILES_DIR="Protos"


%install
rm -rf $RPM_BUILD_ROOT
%cmake_install
find $RPM_BUILD_ROOT -name '*.la' -exec rm -f {} ';'


%post -p /sbin/ldconfig

%postun -p /sbin/ldconfig


%files
%doc
%{_libdir}/*.so.*

%files devel
%doc
%{_includedir}/*
%{_libdir}/*.so


%changelog
