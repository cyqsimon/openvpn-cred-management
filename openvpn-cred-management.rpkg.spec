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

# bin
cargo build --release

# completions
for SHELL in bash zsh fish; do
    target/release/%{_bin_name} gen completion $SHELL > "%{_bin_name}.$SHELL"
done

%install
# bin
install -Dpm 755 target/release/%{_bin_name} %{buildroot}%{_bindir}/%{_bin_name}

# completions
install -Dpm 644 %{_bin_name}.bash %{buildroot}%{_datadir}/bash-completion/completions/%{_bin_name}
install -Dpm 644 %{_bin_name}.zsh %{buildroot}%{_datadir}/zsh/site-functions/_%{_bin_name}
install -Dpm 644 %{_bin_name}.fish %{buildroot}%{_datadir}/fish/completions/%{_bin_name}.fish

%files
%license LICENSE
%{_bindir}/%{_bin_name}
%{_datadir}/bash-completion/completions/%{_bin_name}
%{_datadir}/zsh/site-functions/_%{_bin_name}
%{_datadir}/fish/completions/%{_bin_name}.fish

%changelog
{{{ git_dir_changelog }}}
