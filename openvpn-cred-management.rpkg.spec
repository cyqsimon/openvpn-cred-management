%global debug_package %{nil}
%global _bin_name ocm

Name:       {{{ git_dir_name }}}
Version:    {{{ git_dir_version }}}
Release:    1%{?dist}
Summary:    A wrapper around easy-rsa for personal convenience.
License:    MIT
URL:        https://github.com/cyqsimon/openvpn-cred-management
VCS:        {{{ git_dir_vcs }}}
Source:     {{{ git_dir_pack }}}

Requires:       easy-rsa
BuildRequires:  gcc

%description
A wrapper around easy-rsa for personal convenience.
Not tested whatsoever so you probably shouldn't use it.

%prep
{{{ git_dir_setup_macro }}}

# use latest stable version from rustup
curl -Lf "https://sh.rustup.rs" | sh -s -- --profile minimal -y

%build
source ~/.cargo/env
cargo build --release

%install
install -Dpm 755 target/release/%{_bin_name} %{buildroot}%{_bindir}/%{_bin_name}

%files
%license LICENSE
%{_bindir}/%{_bin_name}

%changelog
{{{ git_dir_changelog }}}
