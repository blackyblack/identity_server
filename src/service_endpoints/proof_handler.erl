-module(proof_handler).

-export([init/2]).

% TODO: add proof tests

init(#{method := <<"POST">>, bindings := #{user := User}} = Req, Opts) ->
    {ok, JsonData, State} = cowboy_req:read_body(Req),
    #{
        <<"signature">> := Signature,
        <<"nonce">> := Nonce,
        <<"signer">> := PublicKey,
        <<"idt">> := Balance,
        <<"proof_id">> := ProofId
    } = json:decode(JsonData),
    RefMessage = identity_signature:proof_signature_message(User, Nonce, Balance, ProofId),
    ok = identity_signature:verify_and_consume_signature(proof, Signature, PublicKey, Nonce, RefMessage),
    ok = identity:set_idt_by_proof(PublicKey, User, Balance, ProofId),
    ProofResp = json:encode(#{
        <<"from">> => PublicKey,
        <<"to">> => User,
        <<"idt">> => Balance,
        <<"proof_id">> => ProofId
    }),
    Resp = cowboy_req:reply(200, #{<<"content-type">> => <<"application/json">>}, ProofResp, State),
    {ok, Resp, Opts}.
