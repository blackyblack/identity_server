%%% @doc
%%% This is the main module that exposes custom formatting to the OTP
%%% logger library (part of the `kernel' application since OTP-21).
%%%
%%% The module honors the standard configuration of the kernel's default
%%% logger formatter regarding: max depth, templates.
%%% @end
-module(logjam_formatter).

%% API exports
-export([apply_defaults/1, format/2, format_log/4, format_to_binary/2, string_to_binary/1]).

-ifdef(TEST).
-export([format_msg/2, to_string/2]).
-endif.

-include_lib("./include/logjam.hrl").

%%====================================================================
%% Internal functions
%%====================================================================
apply_defaults(UserConfig) ->
    DefaultColors = #{
        colored => false,
        colored_date => ?GREEN,
        colored_debug => ?BLUEB,
        colored_info => ?CYAN,
        colored_notice => ?GREENB,
        colored_warning => ?GOLDB,
        colored_error => ?REDB,
        colored_critical => ?RED,
        colored_alert => ?BLACK_ON_GOLD,
        colored_emergency => ?GOLDB_ON_RED,
        colored_pid => ?BLACKB,
        colored_pid_brackets => ?GREEN,
        colored_mfa => ?GOLD,
        colored_arrow => ?CYANB,
        colored_msg => ?GREENB,
        colored_text => ?GREEN},
    Map = maps:merge(DefaultColors, UserConfig),
    #{colored := IsColored} = Map,
    #{colored_mfa := ColoredMfa} = Map,
    #{colored_arrow := ColoredArrow} = Map,
    #{colored_msg := ColoredMsg} = Map,
    Template = case IsColored of
        true -> [time, " ", colored_start, level, colored_end, " ",
                 {id, [" id=", id], ""}, {parent_id, [" parent_id=", parent_id], ""},
                 {correlation_id, [" correlation_id=", correlation_id], ""},
                 pid,
                 " [", ColoredMfa, mfa, ":", line, ?COLOR_END, "] ",
                 ColoredArrow, "▸ ", ?COLOR_END,
                 ColoredMsg, msg, ?COLOR_END, "\n"];
        _ -> [time, " ", colored_start, level, colored_end, " ",
              {id, [" id=", id], ""}, {parent_id, [" parent_id=", parent_id], ""},
              {correlation_id, [" correlation_id=", correlation_id], ""},
              pid,
              " [", mfa, ":", line, "] ", "▸ ", msg, "\n"]
    end,
    maps:merge(
      #{term_depth => undefined,
        map_depth => -1,
        time_offset => 0,
        time_unit => second,
        time_designator => $T,
        strip_tz => false,
        level_capitalize => false,
        level_length => -1,
        template => Template
       },
      Map
    ).

format(#{level := Level, msg := Msg, meta := Metadata}, Config) ->
    format(Msg, Level, Metadata, Config).

format({report, #{format := Format, args := Args}}, Level, Metadata, Config) ->
    format({Format, Args}, Level, Metadata, Config);

format({report, #{report := Report}}, Level, Metadata, Config) when is_list(Report) ->
    format({report, maps:from_list(Report)}, Level, Metadata, Config);

format({report, Msg}, Level, Metadata, Config) when is_map(Msg) ->
    NewConfig = logjam_formatter:apply_defaults(Config),
    #{template := Template} = NewConfig,
    NewMetadata = maps:merge(Metadata, #{
        level => Level,
        colored_start => Level,
        colored_end => ?COLOR_END}),
    logjam_formatter:format_log(Template, NewConfig, Msg, NewMetadata);

format({Format, Args}, Level, Metadata, Config) ->
    format({report, #{text => logjam_formatter:format_to_binary(Format, Args)}}, Level, Metadata, Config).

format_log(Tpl, Config, Msg, Meta) -> format_log(Tpl, Config, Msg, Meta, []).

format_log([], _Config, _Msg, _Meta, Acc) ->
    lists:reverse(Acc);
format_log([msg | Rest], Config, Msg, Meta, Acc) ->
    format_log(Rest, Config, Msg, Meta, [format_msg(Msg, Config) | Acc]);
format_log([Key | Rest], Config, Msg, Meta, Acc) when is_atom(Key)
                                                 orelse is_atom(hd(Key)) -> % from OTP
    case maps:find(Key, Meta) of
        error ->
            format_log(Rest, Config, Msg, Meta, Acc);
        {ok, Val} ->
            format_log(Rest, Config, Msg, Meta, [format_val(Key, Val, Config) | Acc])
    end;
format_log([{Key, IfExists, Else} | Rest], Config, Msg, Meta, Acc) ->
    case maps:find(Key, Meta) of
        error ->
            format_log(Rest, Config, Msg, Meta, [Else | Acc]);
        {ok, Val} ->
            format_log(Rest, Config, Msg, Meta,
                       [format_log(IfExists, Config, Msg, #{Key => Val}, []) | Acc])
    end;
format_log([Term | Rest], Config, Msg, Meta, Acc) when is_list(Term) ->
    format_log(Rest, Config, Msg, Meta, [Term | Acc]).

format_msg(Data, Config) -> format_msg("", Data, Config).

format_msg(Parents, Data, Config=#{map_depth := 0}) when is_map(Data) ->
    to_string(truncate_key(Parents), Config)++"=... ";
format_msg(Parents, Data, Config = #{map_depth := Depth}) when is_map(Data) ->
    maps:fold(
      fun(K, V, Acc) when is_map(V) ->
        [format_msg(Parents ++ to_string(K, Config) ++ "_", V, Config#{map_depth := Depth-1}) | Acc];
        (text, V, Acc) -> [to_string(V, Config), $\s | Acc];
        (K, V, Acc) -> [Parents ++ to_string(K, Config), $=, to_string(V, Config), $\s | Acc]
      end,
      [],
      Data
    ).

format_val(time, Time, Config) ->
    format_time(Time, Config);
format_val(mfa, MFA, Config) ->
    escape(format_mfa(MFA, Config));
format_val(level, Level, Config) ->
    format_level(Level, Config);
format_val(pid, Pid, Config) ->
    format_pid(Pid, Config);
format_val(colored_end, _EOC, #{colored := false}) -> "";
format_val(colored_end, EOC,  #{colored := true}) -> EOC;
format_val(colored_start, _Level,    #{colored := false}) -> "";
format_val(colored_start, debug,     #{colored := true, colored_debug     := BOC}) -> BOC;
format_val(colored_start, info,      #{colored := true, colored_info      := BOC}) -> BOC;
format_val(colored_start, notice,    #{colored := true, colored_notice    := BOC}) -> BOC;
format_val(colored_start, warning,   #{colored := true, colored_warning   := BOC}) -> BOC;
format_val(colored_start, error,     #{colored := true, colored_error     := BOC}) -> BOC;
format_val(colored_start, critical,  #{colored := true, colored_critical  := BOC}) -> BOC;
format_val(colored_start, alert,     #{colored := true, colored_alert     := BOC}) -> BOC;
format_val(colored_start, emergency, #{colored := true, colored_emergency := BOC}) -> BOC;
format_val(_Key, Val, Config) ->
    to_string(Val, Config).

format_time(N, #{time_offset := O,
                 time_unit := U,
                 time_designator := D,
                 strip_tz := Strip}) when is_integer(N) ->
    N2 = case U of
             second -> round(N / 1000000);
             millisecond -> round(N / 1000);
             _ -> N
    end,
    Time = calendar:system_time_to_rfc3339(N2, [{unit, U},
                                                {offset, O},
                                                {time_designator, D}]),
    case Strip of
        true -> lists:sublist(Time, 1, length(Time) - 6);
        _ -> Time
    end.

format_level(Level, Config) when is_atom(Level) ->
    format_level(atom_to_list(Level), Config);
format_level(Level, #{level_capitalize := Is_cap, level_length := Lvl_len}) ->
    L2 = case Is_cap of
        true -> string:to_upper(Level);
        _ -> Level
    end,
    case Lvl_len > 0 of
        true -> lists:sublist(L2, Lvl_len);
        _ -> L2
    end.

format_pid(Pid, Config) when is_pid(Pid) ->
    format_pid(pid_to_list(Pid), Config);
format_pid(Pid, #{colored := false}) when is_list(Pid) -> Pid;
format_pid(Pid, #{colored := true, colored_pid := CP, colored_pid_brackets := CPB}) when is_list(Pid) ->
    CPB ++ "<" ?COLOR_END ++
    CP ++ re:replace(Pid, "[<>]", "", [global, {return, list}]) ++ ?COLOR_END ++
    CPB ++ ">" ++ ?COLOR_END.

format_mfa({M, F, A}, _) when is_atom(M), is_atom(F), is_integer(A) ->
   [atom_to_list(M), $:, atom_to_list(F), $/, integer_to_list(A)];
format_mfa({M, F, A}, Config) when is_atom(M), is_atom(F), is_list(A) ->
    %% arguments are passed as a literal list ({mod, fun, [a, b, c]})
    format_mfa({M, F, length(A)}, Config);
format_mfa(MFAStr, Config) -> % passing in a pre-formatted string value
    re:replace(
        re:replace(escape(to_string(MFAStr,Config)), "^{'", ""),
        "'}$", "").

to_string(X, _) when is_atom(X) ->
    escape(atom_to_list(X));
to_string(X, _) when is_integer(X) ->
    integer_to_list(X);
to_string(X, _) when is_pid(X) ->
    pid_to_list(X);
to_string(X, _) when is_reference(X) ->
    ref_to_list(X);
to_string(X, C = #{colored := IsColored, colored_text := CT}) when is_binary(X) ->
    BeginColor = case IsColored of
        true -> CT;
        _ -> ""
    end,
    EndColor = case IsColored of
        true -> ?COLOR_END;
        _ -> ""
    end,
    String = case unicode:characters_to_list(X) of
        {_, _, _} -> % error or incomplete
            escape(format_str(C, X));
        List ->
            case io_lib:printable_list(List) of
                true -> escape(List);
                _ -> escape(format_str(C, X))
            end
    end,
    BeginColor ++ String ++ EndColor;
to_string(X, C) when is_list(X) ->
    case io_lib:printable_list(X) of
        true -> escape(X);
        _ -> escape(format_str(C, X))
    end;
to_string(X, C) ->
    escape(format_str(C, X)).

format_str(#{term_depth := undefined}, T) ->
    io_lib:format("~0tp", [T]);
format_str(#{term_depth := D}, T) ->
    io_lib:format("~0tP", [T, D]).

escape(Str) -> Str.

truncate_key([]) -> [];
truncate_key("_") -> "";
truncate_key([H|T]) -> [H | truncate_key(T)].

string_to_binary(String) ->
    %% Remove any ANSI colors; this is intended for inputs that have ANSI
    %% colors added to them, e.g., by another logging library/framework.
    T1 = re:replace(String, "\e\[[0-9;]*m", ""),
    unicode:characters_to_binary(T1).

format_to_binary(Format, Terms) ->
    String = io_lib:format(Format, Terms),
    string_to_binary(String).
