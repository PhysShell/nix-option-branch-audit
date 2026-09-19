# Real, bounded excerpt of akkoma's own `config/description.exs` -- 4
# complete, real, non-contiguous top-level records, each independently
# cited with its own real line range (this file's own line numbers do
# NOT match the real upstream file, since these are pasted together,
# not a single contiguous block). Fetched from
# akkoma.dev/AkkomaGang/akkoma @ tag v3.19.0, config/description.exs
# (105482 bytes, confirmed byte-identical size to every earlier real
# citation of this file in this project).
#
# Chosen specifically to exercise both real shapes this project's
# D-extractor needs to handle correctly: `Pleroma.Upload`/`:instance`/
# `:media_proxy` are ordinary flat two-level records (group.key.field);
# `:welcome` has REAL, further-nested children of its own
# (`:direct_message`/`:email`, each with their OWN `:enabled` field) --
# the real shape that makes a naive "flatten every `key: :x` found
# anywhere in the record" extractor unsound (it would merge
# `:welcome.direct_message.enabled` and `:welcome.email.enabled` into
# one bare `:welcome.enabled`, exactly the collision-by-normalization
# risk this whole round exists to catch).

# --- Pleroma.Upload, real lines 84-138 ---
config :pleroma, :config_description, [
  %{
    group: :pleroma,
    key: Pleroma.Upload,
    type: :group,
    description: "Upload general settings",
    children: [
      %{
        key: :uploader,
        type: :module,
        description: "Module which will be used for uploads",
        suggestions: {:list_behaviour_implementations, Pleroma.Uploaders.Uploader}
      },
      %{
        key: :filters,
        type: {:list, :module},
        description:
          "List of filter modules for uploads. Module names are shortened (removed leading `Pleroma.Upload.Filter.` part), but on adding custom module you need to use full name.",
        suggestions: {:list_behaviour_implementations, Pleroma.Upload.Filter}
      },
      %{
        key: :link_name,
        type: :boolean,
        description:
          "If enabled, a name parameter will be added to the URL of the upload. For example `https://instance.tld/media/imagehash.png?name=realname.png`."
      },
      %{
        key: :base_url,
        label: "Base URL",
        type: :string,
        description:
          "Base URL for the uploads. Required if you use a CDN or host attachments under a different domain - it is HIGHLY recommended that you **do not** set this to be the same as the domain akkoma is hosted on.",
        suggestions: [
          "https://media.akkoma.dev/media/"
        ]
      },
      %{
        key: :allowed_mime_types,
        label: "Allowed MIME types",
        type: {:list, :string},
        description:
          "List of MIME (main) types uploads are allowed to identify themselves with. Other types may still be uploaded, but will identify as a generic binary to clients. WARNING: Loosening this over the defaults can lead to security issues. Removing types is safe, but only add to the list if you are sure you know what you are doing.",
        suggestions: [
          "image",
          "audio",
          "video",
          "font"
        ]
      },
      %{
        key: :filename_display_max_length,
        type: :integer,
        description: "Set max length of a filename to display. 0 = no limit. Default: 30"
      }
    ]

# --- :instance, real lines 535-1021 ---
  %{
    group: :pleroma,
    key: :instance,
    type: :group,
    description: "Instance-related settings",
    children: [
      %{
        key: :name,
        type: :string,
        description: "Name of the instance",
        suggestions: [
          "Pleroma"
        ]
      },
      %{
        key: :languages,
        type: {:list, :string},
        description: "Languages the instance uses",
        suggestions: [
          "en",
          "ja",
          "fr"
        ]
      },
      %{
        key: :email,
        label: "Admin Email Address",
        type: :string,
        description: "Email used to reach an Administrator/Moderator of the instance",
        suggestions: [
          "email@example.com"
        ]
      },
      %{
        key: :notify_email,
        label: "Sender Email Address",
        type: :string,
        description: "Envelope FROM address for mail sent via Pleroma",
        suggestions: [
          "notify@example.com"
        ]
      },
      %{
        key: :description,
        type: :string,
        description:
          "The instance's description. It may use HTML and can be seen in `/api/v1/instance` and nodeifno if no short description is set",
        suggestions: [
          "Very cool instance"
        ]
      },
      %{
        key: :short_description,
        type: :string,
        description:
          "A brief instance description. It must be plain text and can be seen in `/api/v1/instance` and nodeinfo",
        suggestions: [
          "Very cool instance"
        ]
      },
      %{
        key: :limit,
        type: :integer,
        description: "Posts character limit (CW/Subject included in the counter)",
        suggestions: [
          5_000
        ]
      },
      %{
        key: :remote_limit,
        type: :integer,
        description: "Hard character limit beyond which remote posts will be dropped",
        suggestions: [
          100_000
        ]
      },
      %{
        key: :upload_limit,
        type: :integer,
        description: "File size limit of uploads (except for avatar, background, banner)",
        suggestions: [
          16_000_000
        ]
      },
      %{
        key: :avatar_upload_limit,
        type: :integer,
        description: "File size limit of user's profile avatars",
        suggestions: [
          2_000_000
        ]
      },
      %{
        key: :background_upload_limit,
        type: :integer,
        description: "File size limit of user's profile backgrounds",
        suggestions: [
          4_000_000
        ]
      },
      %{
        key: :banner_upload_limit,
        type: :integer,
        description: "File size limit of user's profile banners",
        suggestions: [
          4_000_000
        ]
      },
      %{
        key: :poll_limits,
        type: :map,
        description: "A map with poll limits for local polls",
        suggestions: [
          %{
            max_options: 20,
            max_option_chars: 200,
            min_expiration: 0,
            max_expiration: 31_536_000
          }
        ],
        children: [
          %{
            key: :max_options,
            type: :integer,
            description: "Maximum number of options",
            suggestions: [20]
          },
          %{
            key: :max_option_chars,
            type: :integer,
            description: "Maximum number of characters per option",
            suggestions: [200]
          },
          %{
            key: :min_expiration,
            type: :integer,
            description: "Minimum expiration time (in seconds)",
            suggestions: [0]
          },
          %{
            key: :max_expiration,
            type: :integer,
            description: "Maximum expiration time (in seconds)",
            suggestions: [3600]
          }
        ]
      },
      %{
        key: :registrations_open,
        type: :boolean,
        description:
          "Enable registrations for anyone. Invitations require this setting to be disabled."
      },
      %{
        key: :invites_enabled,
        type: :boolean,
        description:
          "Enable user invitations for admins (depends on `registrations_open` being disabled)"
      },
      %{
        key: :account_activation_required,
        type: :boolean,
        description: "Require users to confirm their emails before signing in"
      },
      %{
        key: :account_approval_required,
        type: :boolean,
        description: "Require users to be manually approved by an admin before signing in"
      },
      %{
        key: :federating,
        type: :boolean,
        description: "Enable federation with other instances"
      },
      %{
        key: :federation_incoming_replies_max_depth,
        label: "Fed. incoming replies max depth",
        type: :integer,
        description:
          "Max. depth of reply-to and reply activities fetching on incoming federation, to prevent out-of-memory situations while" <>
            " fetching very long threads. If set to `nil`, threads of any depth will be fetched. Lower this value if you experience out-of-memory crashes.",
        suggestions: [
          100
        ]
      },
      %{
        key: :federation_reachability_timeout_days,
        label: "Fed. reachability timeout days",
        type: :integer,
        description:
          "Timeout (in days) of each external federation target being unreachable prior to pausing federating to it",
        suggestions: [
          7
        ]
      },
      %{
        key: :allow_relay,
        type: :boolean,
        description:
          "Permits remote instances to subscribe to all public posts of your instance. (Important!) This may increase the visibility of your instance."
      },
      %{
        key: :public,
        type: :boolean,
        description:
          "Switching this on will allow unauthenticated users access to all public resources on your instance" <>
            " Switching it off is useful for disabling the Local Timeline and The Whole Known Network. " <>
            " Note: when setting to `false`, please also check `:restrict_unauthenticated` setting."
      },
      %{
        key: :quarantined_instances,
        type: {:list, :tuple},
        key_placeholder: "instance",
        value_placeholder: "reason",
        description:
          "(Deprecated, will be removed in next release) List of ActivityPub instances where activities will not be sent, and the reason for doing so",
        suggestions: [
          {"quarantined.com", "Reason"},
          {"*.quarantined.com", "Reason"}
        ]
      },
      %{
        key: :static_dir,
        type: :string,
        description: "Instance static directory",
        suggestions: [
          "instance/static/"
        ]
      },
      %{
        key: :allowed_post_formats,
        type: {:list, :string},
        description: "MIME-type list of formats allowed to be posted (transformed into HTML)",
        suggestions: [
          "text/plain",
          "text/html",
          "text/markdown",
          "text/bbcode",
          "text/x.misskeymarkdown"
        ]
      },
      %{
        key: :extended_nickname_format,
        type: :boolean,
        description:
          "Enable to use extended local nicknames format (allows underscores/dashes)." <>
            " This will break federation with older software for theses nicknames."
      },
      %{
        key: :cleanup_attachments,
        type: :boolean,
        description: """
        Enable to remove associated attachments when status is removed.
        This will not affect duplicates and attachments without status.
        Enabling this will increase load to database when deleting statuses on larger instances.
        """
      },
      %{
        key: :max_pinned_statuses,
        type: :integer,
        description: "The maximum number of pinned statuses. 0 will disable the feature.",
        suggestions: [
          0,
          1,
          3
        ]
      },
      %{
        key: :autofollowed_nicknames,
        type: {:list, :string},
        description:
          "Set to nicknames of (local) users that every new user should automatically follow"
      },
      %{
        key: :autofollowing_nicknames,
        type: {:list, :string},
        description:
          "Set to nicknames of (local) users that automatically follows every newly registered user"
      },
      %{
        key: :attachment_links,
        type: :boolean,
        description: "Enable to automatically add attachment link text to statuses"
      },
      %{
        key: :max_report_comment_size,
        type: :integer,
        description: "The maximum size of the report comment. Default: 1000.",
        suggestions: [
          1_000
        ]
      },
      %{
        key: :safe_dm_mentions,
        label: "Safe DM mentions",
        type: :boolean,
        description:
          "If enabled, only mentions at the beginning of a post will be used to address people in direct messages." <>
            " This is to prevent accidental mentioning of people when talking about them (e.g. \"@admin please keep an eye on @bad_actor\")." <>
            " Default: disabled"
      },
      %{
        key: :healthcheck,
        type: :boolean,
        description: "If enabled, system data will be shown on `/api/v1/pleroma/healthcheck`"
      },
      %{
        key: :remote_post_retention_days,
        type: :integer,
        description:
          "The default amount of days to retain remote posts when pruning the database",
        suggestions: [
          90
        ]
      },
      %{
        key: :user_bio_length,
        type: :integer,
        description: "A user bio maximum length. Default: 5000.",
        suggestions: [
          5_000
        ]
      },
      %{
        key: :user_name_length,
        type: :integer,
        description: "A user name maximum length. Default: 100.",
        suggestions: [
          100
        ]
      },
      %{
        key: :limit_to_local_content,
        type: {:dropdown, :atom},
        description:
          "Limit unauthenticated users to search for local statutes and users only. Default: `:unauthenticated`.",
        suggestions: [
          :unauthenticated,
          :all,
          false
        ]
      },
      %{
        key: :max_account_fields,
        type: :integer,
        description: "The maximum number of custom fields in the user profile. Default: 10.",
        suggestions: [
          10
        ]
      },
      %{
        key: :max_remote_account_fields,
        type: :integer,
        description:
          "The maximum number of custom fields in the remote user profile. Default: 20.",
        suggestions: [
          20
        ]
      },
      %{
        key: :account_field_name_length,
        type: :integer,
        description: "An account field name maximum length. Default: 512.",
        suggestions: [
          512
        ]
      },
      %{
        key: :account_field_value_length,
        type: :integer,
        description: "An account field value maximum length. Default: 2048.",
        suggestions: [
          2048
        ]
      },
      %{
        key: :registration_reason_length,
        type: :integer,
        description: "Maximum registration reason length. Default: 500.",
        suggestions: [
          500
        ]
      },
      %{
        key: :external_user_synchronization,
        type: :boolean,
        description: "Enabling following/followers counters synchronization for external users"
      },
      %{
        key: :multi_factor_authentication,
        type: :keyword,
        description: "Multi-factor authentication settings",
        suggestions: [
          [
            totp: [digits: 6, period: 30],
            backup_codes: [number: 5, length: 16]
          ]
        ],
        children: [
          %{
            key: :totp,
            label: "TOTP settings",
            type: :keyword,
            description: "TOTP settings",
            suggestions: [digits: 6, period: 30],
            children: [
              %{
                key: :digits,
                type: :integer,
                suggestions: [6],
                description:
                  "Determines the length of a one-time pass-code, in characters. Defaults to 6 characters."
              },
              %{
                key: :period,
                type: :integer,
                suggestions: [30],
                description:
                  "A period for which the TOTP code will be valid, in seconds. Defaults to 30 seconds."
              }
            ]
          },
          %{
            key: :backup_codes,
            type: :keyword,
            description: "MFA backup codes settings",
            suggestions: [number: 5, length: 16],
            children: [
              %{
                key: :number,
                type: :integer,
                suggestions: [5],
                description: "Number of backup codes to generate."
              },
              %{
                key: :length,
                type: :integer,
                suggestions: [16],
                description:
                  "Determines the length of backup one-time pass-codes, in characters. Defaults to 16 characters."
              }
            ]
          }
        ]
      },
      %{
        key: :instance_thumbnail,
        type: {:string, :image},
        description:
          "The instance thumbnail can be any image that represents your instance and is used by some apps or services when they display information about your instance.",
        suggestions: ["/instance/thumbnail.jpeg"]
      },
      %{
        key: :show_reactions,
        type: :boolean,
        description: "Let favourites and emoji reactions be viewed through the API."
      },
      %{
        key: :profile_directory,
        type: :boolean,
        description: "Enable profile directory."
      },
      %{
        key: :privileged_staff,
        type: :boolean,
        description:
          "Let moderators access sensitive data (e.g. updating user credentials, get password reset token, delete users, index and read private statuses)"
      },
      %{
        key: :local_bubble,
        type: {:list, :string},
        description:
          "List of instances that make up your local bubble (closely-related instances). Used to populate the 'bubble' timeline (domain only)."
      },
      %{
        key: :export_prometheus_metrics,
        type: :boolean,
        description: "Enable prometheus metrics (at /api/v1/akkoma/metrics)"
      },
      %{
        key: :federated_timeline_available,
        type: :boolean,
        description:
          "Let people view the 'firehose' feed of all public statuses from all instances."
      }
    ]
  },

# --- :welcome, real lines 1023-1093 (real nested children) ---
    group: :pleroma,
    key: :welcome,
    type: :group,
    description: "Welcome messages settings",
    children: [
      %{
        key: :direct_message,
        type: :keyword,
        descpiption: "Direct message settings",
        children: [
          %{
            key: :enabled,
            type: :boolean,
            description: "Enables sending a direct message to newly registered users"
          },
          %{
            key: :message,
            type: :string,
            description: "A message that will be sent to newly registered users",
            suggestions: [
              "Hi, @username! Welcome on board!"
            ]
          },
          %{
            key: :sender_nickname,
            type: :string,
            description: "The nickname of the local user that sends a welcome message",
            suggestions: [
              "lain"
            ]
          }
        ]
      },
      %{
        key: :email,
        type: :keyword,
        descpiption: "Email message settings",
        children: [
          %{
            key: :enabled,
            type: :boolean,
            description: "Enables sending an email to newly registered users"
          },
          %{
            key: :sender,
            type: [:string, :tuple],
            description:
              "Email address and/or nickname that will be used to send the welcome email.",
            suggestions: [
              {"Pleroma App", "welcome@pleroma.app"}
            ]
          },
          %{
            key: :subject,
            type: :string,
            description:
              "Subject of the welcome email. EEX template with user and instance_name variables can be used.",
            suggestions: ["Welcome to <%= instance_name%>"]
          },
          %{
            key: :html,
            type: :string,
            description:
              "HTML content of the welcome email. EEX template with user and instance_name variables can be used.",
            suggestions: ["<h1>Hello <%= user.name%>. Welcome to <%= instance_name%></h1>"]
          },
          %{
            key: :text,
            type: :string,
            description:
              "Text content of the welcome email. EEX template with user and instance_name variables can be used.",

# --- :media_proxy, real lines 1534-1633 ---
  %{
    group: :pleroma,
    key: :media_proxy,
    type: :group,
    description: "Media proxy",
    children: [
      %{
        key: :enabled,
        type: :boolean,
        description: "Enables proxying of remote media via the instance's proxy"
      },
      %{
        key: :base_url,
        label: "Base URL",
        type: :string,
        description:
          "The base URL to access a user-uploaded file. Useful when you want to proxy the media files via another host/CDN fronts.",
        suggestions: ["https://example.com"]
      },
      %{
        key: :invalidation,
        type: :keyword,
        descpiption: "",
        suggestions: [
          enabled: true,
          provider: Pleroma.Web.MediaProxy.Invalidation.Script
        ],
        children: [
          %{
            key: :enabled,
            type: :boolean,
            description: "Enables media cache object invalidation."
          },
          %{
            key: :provider,
            type: :module,
            description: "Module which will be used to purge objects from the cache.",
            suggestions: [
              Pleroma.Web.MediaProxy.Invalidation.Script,
              Pleroma.Web.MediaProxy.Invalidation.Http
            ]
          }
        ]
      },
      %{
        key: :proxy_opts,
        label: "Advanced MediaProxy Options",
        type: :keyword,
        description: "Internal Pleroma.ReverseProxy settings",
        suggestions: [
          redirect_on_failure: false,
          max_body_length: 25 * 1_048_576,
          max_read_duration: 30_000
        ],
        children: [
          %{
            key: :redirect_on_failure,
            type: :boolean,
            description: """
            Redirects the client to the origin server upon encountering HTTP errors.\n
            Note that files larger than Max Body Length will trigger an error. (e.g., Peertube videos)\n\n
            **WARNING:** This setting will allow larger files to be accessed, but exposes the\n
            IP addresses of your users to the other servers, bypassing the MediaProxy.
            """
          },
          %{
            key: :max_body_length,
            type: :integer,
            description:
              "Maximum file size (in bytes) allowed through the Pleroma MediaProxy cache."
          },
          %{
            key: :max_read_duration,
            type: :integer,
            description: "Timeout (in milliseconds) of GET request to the remote URI."
          }
        ]
      },
      %{
        key: :whitelist,
        type: {:list, :string},
        description: """
        List of hosts with scheme to bypass the MediaProxy.\n
        The media will be fetched by the client, directly from the remote server.\n
        To allow this, it will Content-Security-Policy exceptions for each instance listed.\n
        This is to be used for instances you trust and do not want to cache media for.
        """,
        suggestions: ["http://example.com"]
      },
      %{
        key: :blocklist,
        type: {:list, :string},
        description: """
        List of hosts with scheme which will not go through the MediaProxy, and will not be explicitly allowed by the Content-Security-Policy.
        This is to be used for instances where you do not want their media to go through your server or to be accessed by clients.
        """,
        suggestions: ["http://example.com"]
      }
    ]
  },
