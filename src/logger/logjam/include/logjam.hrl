%%%-----------------------------------------------------------------
%%% Convenience definitions
%%% 
-define(BLACK, "\e[0;30m").
-define(BLACKB, "\e[1;30m").
-define(BLACK_ON_GOLD, "\e[30;43m").
-define(BLUE, "\e[0;34m").
-define(BLUEB, "\e[1;34m").
-define(CYAN, "\e[0;36m").
-define(CYANB, "\e[1;36m").
-define(GOLD, "\e[0;33m").
-define(GOLDB, "\e[1;33m").
-define(GOLDB_ON_RED, "\e[1;33;41m").
-define(GREEN, "\e[0;32m").
-define(GREENB, "\e[1;32m").
-define(GREY, "\e[0;37m").
-define(GREYB, "\e[1;37m").
-define(MAGENTA, "\e[0;35m").
-define(MAGENTAB, "\e[1;35m").
-define(RED, "\e[0;31m").
-define(REDB, "\e[1;31m").
-define(COLOR_END, "\e[0m").

-define(LOCATION,#{mfa=>{?MODULE,?FUNCTION_NAME,?FUNCTION_ARITY},
                   line=>?LINE,
                   file=>?FILE}).

%%%-----------------------------------------------------------------
%%% Internal, i.e. not intended for direct use in code - use above
%%% macros instead!
-define(DO_LOG(Level,Args),
    case logger:allow(Level,?MODULE) of
        true -> apply(logger,macro_log,[?LOCATION,Level|Args]);
        false -> ok
    end
).
