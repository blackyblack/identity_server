-module(moderators_handler).

-export([init/2]).

init(#{method := <<"GET">>} = Req, Opts) ->
    Moders = moderators:moderators_list(),
    ModersResp = json:encode(Moders),
    Resp = cowboy_req:reply(200, #{<<"content-type">> => <<"application/json">>}, ModersResp, Req),
    {ok, Resp, Opts}.
