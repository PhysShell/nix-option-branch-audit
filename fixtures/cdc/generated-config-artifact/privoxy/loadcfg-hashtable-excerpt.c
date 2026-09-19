/* Real, bounded excerpt of privoxy's own `#define hash_X ... /* "name" */`
 * directive-name table -- fetched from the real upstream source at tag
 * v4.2.0-stable (sourceforge.net/projects/ijbswa/files, mirrored via
 * github.com/ojrbliss/privoxy), `loadcfg.c` lines 130-200 verbatim.
 * A single flat, global table (loadcfg.c has exactly ONE
 * `switch(hash_string(cmd))` dispatch fed by this table, confirmed by
 * reading the whole file directly -- no per-section sub-dispatch
 * anywhere), matching this fixture's own real citation exactly.
 */
 */

#define hash_actions_file                1196306641U /* "actionsfile" */
#define hash_accept_intercepted_requests 1513024973U /* "accept-intercepted-requests" */
#define hash_admin_address               4112573064U /* "admin-address" */
#define hash_allow_cgi_request_crunching  258915987U /* "allow-cgi-request-crunching" */
#define hash_buffer_limit                1881726070U /* "buffer-limit */
#define hash_ca_cert_file                1622923720U /* "ca-cert-file" */
#define hash_ca_directory                1623615670U /* "ca-directory" */
#define hash_ca_key_file                 1184187891U /* "ca-key-file" */
#define hash_ca_password                 1184543320U /* "ca-password" */
#define hash_certificate_directory       1367994217U /* "certificate-directory" */
#define hash_cipher_list                 1225729316U /* "cipher-list" */
#define hash_client_header_order         2701453514U /* "client-header-order" */
#define hash_client_specific_tag         3353703383U /* "client-specific-tag" */
#define hash_client_tag_lifetime         3239141416U /* "client-tag-lifetime" */
#define hash_compression_level           2464423563U /* "compression-level" */
#define hash_confdir                        1978389U /* "confdir" */
#define hash_connection_sharing          1348841265U /* "connection-sharing" */
#define hash_cors_allowed_origin         2769345637U /* "cors-allowed-origin" */
#define hash_debug                            78263U /* "debug" */
#define hash_default_server_timeout      2530089913U /* "default-server-timeout" */
#define hash_deny_access                 1227333715U /* "deny-access" */
#define hash_elliptic_curve_keys          258906537U /* "elliptic-curve-keys" */
#define hash_enable_accept_filter        2909040407U /* "enable-accept-filter" */
#define hash_enable_edit_actions         2517097536U /* "enable-edit-actions" */
#define hash_enable_compression          3943696946U /* "enable-compression" */
#define hash_enable_proxy_authentication_forwarding 4040610791U /* enable-proxy-authentication-forwarding */
#define hash_enable_remote_toggle        2979744683U /* "enable-remote-toggle" */
#define hash_enable_remote_http_toggle    110543988U /* "enable-remote-http-toggle" */
#define hash_enforce_blocks              1862427469U /* "enforce-blocks" */
#define hash_filterfile                   250887266U /* "filterfile" */
#define hash_forward                        2029845U /* "forward" */
#define hash_forward_socks4              3963965521U /* "forward-socks4" */
#define hash_forward_socks4a             2639958518U /* "forward-socks4a" */
#define hash_forward_socks5              3963965522U /* "forward-socks5" */
#define hash_forward_socks5t             2639958542U /* "forward-socks5t" */
#define hash_forwarded_connect_retries    101465292U /* "forwarded-connect-retries" */
#define hash_handle_as_empty_returns_ok  1444873247U /* "handle-as-empty-doc-returns-ok" */
#define hash_hostname                      10308071U /* "hostname" */
#define hash_keep_alive_timeout          3878599515U /* "keep-alive-timeout" */
#define hash_listen_address              1255650842U /* "listen-address" */
#define hash_listen_backlog              1255655735U /* "listen-backlog" */
#define hash_logdir                          422889U /* "logdir" */
#define hash_logfile                        2114766U /* "logfile" */
#define hash_max_client_connections      3595884446U /* "max-client-connections" */
#define hash_permit_access               3587953268U /* "permit-access" */
#define hash_proxy_info_url              3903079059U /* "proxy-info-url" */
#define hash_receive_buffer_size         2880297454U /* "receive-buffer-size */
#define hash_single_threaded             4250084780U /* "single-threaded" */
#define hash_socket_timeout              1809001761U /* "socket-timeout" */
#define hash_split_large_cgi_forms        671658948U /* "split-large-cgi-forms" */
#define hash_suppress_blocklists         1948693308U /* "suppress-blocklists" */
#define hash_templdir                      11067889U /* "templdir" */
#define hash_temporary_directory         1824125181U /* "temporary-directory" */
#define hash_tolerate_pipelining         1360286620U /* "tolerate-pipelining" */
#define hash_toggle                          447966U /* "toggle" */
#define hash_trust_info_url               430331967U /* "trust-info-url" */
#define hash_trust_x_forwarded_for       2971537414U /* "trust-x-forwarded-for" */
#define hash_trusted_cgi_referrer        4270883427U /* "trusted-cgi-referrer" */
#define hash_trusted_cas_file            2679803024U /* "trusted-cas-files" */
#define hash_trustfile                     56494766U /* "trustfile" */
#define hash_usermanual                  1416668518U /* "user-manual" */
#define hash_activity_animation          1817904738U /* "activity-animation" */
#define hash_close_button_minimizes      3651284693U /* "close-button-minimizes" */
#define hash_hide_console                2048809870U /* "hide-console" */
#define hash_log_buffer_size             2918070425U /* "log-buffer-size" */
#define hash_log_font_name               2866730124U /* "log-font-name" */
#define hash_log_font_size               2866731014U /* "log-font-size" */
#define hash_log_highlight_messages      4032101240U /* "log-highlight-messages" */
#define hash_log_max_lines               2868344173U /* "log-max-lines" */
#define hash_log_messages                2291744899U /* "log-messages" */
#define hash_show_on_task_bar             215410365U /* "show-on-task-bar" */

