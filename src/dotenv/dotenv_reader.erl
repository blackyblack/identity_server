-module(dotenv_reader).

-export([read/1]).

-include("./include/dotenv.hrl").

-spec read(file:name_all()) -> {ok, config()} | {error, any()}.
read(FileName) ->
    maybe
        {ok, FileContent} ?= file:read_file(FileName),
        Lines = binary:split(FileContent, <<"\n">>, [global, trim_all]),
        parse_lines(Lines, #{})
    end.

parse_lines([], Config) ->
    {ok, Config};

parse_lines([H | T], Config) ->
    case parse_line(H) of
        {single_line, Key, Value} ->
            NewConfig = maps:put(Key, Value, Config),
            parse_lines(T, NewConfig);
        {multi_line, Key, Value} ->
            parse_lines_continue(T, Config, Key, Value);
        error -> {error, H}
    end.

parse_lines_continue([H | T], Config, Key, MultilineValue) ->
    case parse_line_continue(H, MultilineValue) of
        {stop, Value} ->
            NewConfig = maps:put(Key, Value, Config),
            parse_lines(T, NewConfig);
        {continue, Value} ->
            parse_lines_continue(T, Config, Key, Value);
        error -> {error, H}
    end.

parse_line(Line) ->
    case binary:split(Line, <<"=">>, [trim_all]) of
        [Key, Value] ->
            {LineType, ParsedValue} = parse_value(string:trim(Value)),
            {LineType, string:trim(Key), ParsedValue};
        _ -> error
    end.

% For multiline parsing
parse_line_continue(Line, MultilineValue) ->
    Trimmed = binary_to_list(string:trim(Line)),
    {H, T} = {lists:droplast(Trimmed), lists:last(Trimmed)},
    case <<T>> of
        <<"\"">> ->
            LineStart = list_to_binary(H),
            {stop, <<MultilineValue/binary, "\n", LineStart/binary>>};
        _ ->
            FullLine = list_to_binary(Trimmed),
            {continue, <<MultilineValue/binary, "\n", FullLine/binary>>}
    end.

parse_value(<<"\"", Value/binary>>) ->
    {multi_line, Value};

parse_value(Value) ->
    {single_line, Value}.
