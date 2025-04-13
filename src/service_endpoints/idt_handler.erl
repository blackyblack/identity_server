-module(idt_handler).

-export([init/2]).

init(#{method := <<"GET">>, bindings := #{user := User}} = Req, Opts) ->
    Idt = identity:idt(User),
    IdtResp = json:encode(#{<<"idt">> => Idt}),
    Resp = cowboy_req:reply(200, #{<<"content-type">> => <<"application/json">>}, IdtResp, Req),
    {ok, Resp, Opts}.
