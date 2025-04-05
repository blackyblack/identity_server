-module(vouch_handler).

-export([init/2]).

init(#{method := <<"POST">>, bindings := #{user := UserEncoded}} = Req, Opts) ->
    {ok, JsonData, State} = cowboy_req:read_body(Req),
    #{<<"signature">> := SignatureEncoded, <<"nonce">> := Nonce, <<"signer">> := PublicKeyEncoded} = json:decode(JsonData),
    RefMessage = identity_signature:message_for_signature(vouch, PublicKeyEncoded, Nonce),
    ok = identity_signature:verify_and_consume_signature(SignatureEncoded, PublicKeyEncoded, Nonce, RefMessage),
    {ok, VouchResp} = identity:vouch(PublicKeyEncoded, UserEncoded),
    Resp = cowboy_req:reply(200, #{<<"content-type">> => <<"application/json">>}, VouchResp, State),
    {ok, Resp, Opts}.
