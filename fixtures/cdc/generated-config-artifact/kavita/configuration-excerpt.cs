/* Real, bounded excerpt of Kavita's own real accepted config schema --
 * fetched from Kareadita/Kavita @ tag v0.9.0.2,
 * Kavita.Common/Configuration.cs, lines 375-404 verbatim: the real
 * `AppSettings` sealed class (top-level accepted keys) and the real
 * nested `OpenIdConnectSettings` class it references, both fully-typed
 * public C# properties. Note the real accessor mix: most are
 * `{ get; set; }`, but `AllowIFraming` is `{ get; init; }` -- a real
 * shape a bounded extractor must recognize both forms of.
 */
    private sealed class AppSettings
    {
        public string TokenKey { get; set; }
        // ReSharper disable once MemberHidesStaticFromOuterClass
#pragma warning disable S3218
        public int Port { get; set; } = DefaultHttpPort;
        // ReSharper disable once MemberHidesStaticFromOuterClass
        public string IpAddresses { get; set; } = string.Empty;
        // ReSharper disable once MemberHidesStaticFromOuterClass
        public string BaseUrl { get; set; }
        // ReSharper disable once MemberHidesStaticFromOuterClass
        public long Cache { get; set; } = DefaultCacheMemory;
        // ReSharper disable once MemberHidesStaticFromOuterClass
        public bool AllowIFraming { get; init; } = false;
        public OpenIdConnectSettings OpenIdConnectSettings { get; set; } = new();
#pragma warning restore S3218
    }

    public class OpenIdConnectSettings
    {
        public string Authority { get; set; } = DefaultOidcAuthority;
        public string ClientId { get; set; } = DefaultOidcClientId;
        public string Secret { get; set; } = string.Empty;
        public List<string> CustomScopes { get; set; } = [];

        public bool Enabled =>
            !string.IsNullOrEmpty(Authority) &&
            !string.IsNullOrEmpty(ClientId) &&
            !string.IsNullOrEmpty(Secret);
    }
