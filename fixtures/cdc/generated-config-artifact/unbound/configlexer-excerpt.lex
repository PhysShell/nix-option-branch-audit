/* Bounded, real, vendored excerpt -- NLnetLabs/unbound, tag release-1.26.0,
 * commit a45da353d3feb5d8fc00685fa1ceda3816d5108f (confirmed matching the
 * real pinned nixpkgs version 1.26.0). NOT the complete grammar/lexer --
 * unbound's own real accepted-config surface is hundreds of directives;
 * this excerpt covers exactly the `server`/`remote-control` directives
 * this project's own real test config (fixtures/cdc/generated-config-
 * artifact/unbound/rendered.conf) emits, real line numbers cited from
 * each real source file, not invented or renumbered.
 *
 * util/configlexer.lex -- keyword -> yacc token mapping (each line's own
 * real source line number in that file, cited inline):
 */

/* configlexer.lex:221 */
port{COLON}			{ YDVAR(1, VAR_PORT) }
/* configlexer.lex:253 */
tls-cert-bundle{COLON}		{ YDVAR(1, VAR_TLS_CERT_BUNDLE) }
/* configlexer.lex:275 */
do-daemonize{COLON}		{ YDVAR(1, VAR_DO_DAEMONIZE) }
/* configlexer.lex:276 */
interface{COLON}		{ YDVAR(1, VAR_INTERFACE) }
/* configlexer.lex:285 */
ip-freebind{COLON}		{ YDVAR(1, VAR_IP_FREEBIND) }
/* configlexer.lex:287 */
chroot{COLON}			{ YDVAR(1, VAR_CHROOT) }
/* configlexer.lex:288 */
username{COLON}			{ YDVAR(1, VAR_USERNAME) }
/* configlexer.lex:289 */
directory{COLON}		{ YDVAR(1, VAR_DIRECTORY) }
/* configlexer.lex:291 */
pidfile{COLON}			{ YDVAR(1, VAR_PIDFILE) }
/* configlexer.lex:373 */
access-control{COLON}		{ YDVAR(2, VAR_ACCESS_CONTROL) }
/* configlexer.lex:396 */
auto-trust-anchor-file{COLON}	{ YDVAR(1, VAR_AUTO_TRUST_ANCHOR_FILE) }
/* configlexer.lex:456 */
control-enable{COLON}		{ YDVAR(1, VAR_CONTROL_ENABLE) }
/* configlexer.lex:457 */
control-interface{COLON}	{ YDVAR(1, VAR_CONTROL_INTERFACE) }
/* configlexer.lex:460 */
server-key-file{COLON}		{ YDVAR(1, VAR_SERVER_KEY_FILE) }
/* configlexer.lex:461 */
server-cert-file{COLON}		{ YDVAR(1, VAR_SERVER_CERT_FILE) }
/* configlexer.lex:462 */
control-key-file{COLON}		{ YDVAR(1, VAR_CONTROL_KEY_FILE) }
/* configlexer.lex:463 */
control-cert-file{COLON}	{ YDVAR(1, VAR_CONTROL_CERT_FILE) }
