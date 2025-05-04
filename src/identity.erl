-module(identity).

-export([vouch/2, idt/1, penalty/1, set_idt_by_proof/4, set_idt_by_config/2, punish/4]).

-define(TOP_VOUCHERS_SIZE, 5).
-define(MAX_IDT_BY_PROOF, 50000).
% allows to ban for twice the entire balance, i.e. permanent ban. However due to penalty decay
% IDT balance can eventually become positive.
% It only limits vouchee penalty because we do not want to limit amount of penalties and their value
% for a single user but we do not want to propagate it across the network indefinitely. 
-define(MAX_VOUCHEE_PENALTY, ?MAX_IDT_BY_PROOF * 4).
% reduce IDT by this coefficient for vouchee. So each level of vouch inherits voucher balance multiplied by this coefficient.
-define(IDT_REDUCE_BY_LEVEL_COEFFICIENT, 0.1).
% reduce IDT penalty by this coefficient for voucher. So each level of vouch inherits vouchee penalty multiplied by this coefficient.
-define(PENALTY_REDUCE_BY_LEVEL_COEFFICIENT, 0.1).
% this proof id is set for initial user balances
-define(GENESIS_PROOF_ID, <<"0">>).

%TODO: might require to migrate to gen_server to make IDT updates atomic

vouch(From, To) ->
    ets:insert(vouches, {{To, From}, erlang:monotonic_time()}),
    {ok, idt(To)}.

% IDT calculation requires iterating all way down the graph so it can take
% substantial time and can be affected by graph reorganizations during
% traversal
idt(User) ->
    {Idt, _} = idt_visited(User, sets:new()),
    Idt.

penalty(User) ->
    {Idt, _} = penalty_visited(User, sets:new()),
    Idt.

% Should be called by moderator after proof verification.
% Proof should be publicly verifiable, probably with ZK proof.
set_idt_by_proof(Moderator, User, Balance, ProofId) ->
    maybe
        ok ?= case moderators:is_moderator(Moderator) of
            false -> {error, not_allowed};
            _ -> ok
        end,
        ok ?= case Balance > ?MAX_IDT_BY_PROOF of
            true -> {error, max_balance_exceeded};
            _ -> ok
        end,
        ets:insert(id_proofs, {User, Balance, erlang:monotonic_time(), ProofId}),
        ok
    end.

set_idt_by_config(User, Balance) ->
    ets:insert(id_proofs, {User, Balance, erlang:monotonic_time(), ?GENESIS_PROOF_ID}).

punish(Moderator, To, Balance, ProofId) ->
    maybe
        ok ?= case moderators:is_moderator(Moderator) of
            false -> {error, not_allowed};
            _ -> ok
        end,
        % no balance check on punishment. Keep it at moderators' discretion.
        ets:insert(penalties, {ProofId, To, Moderator, Balance, erlang:monotonic_time()}),
        {ok, idt(To)}
    end.

idt_visited(User, Visited) ->
    case sets:is_element(User, Visited) of
        true -> {0, Visited};
        _ ->
            VisitedUser = sets:add_element(User, Visited),
            TopVouchers = top_vouchers(User, VisitedUser),
            BalanceByVouchers = user_idt_from_vouchers(TopVouchers),
            BalanceByProof = user_idt_from_proof(User),
            Penalty = penalty(User),
            ResultingBalance = case BalanceByVouchers + BalanceByProof > Penalty of
                true -> BalanceByVouchers + BalanceByProof - Penalty;
                _ -> 0
            end,
            {ResultingBalance, VisitedUser}
    end.

% returns a list of top vouchers with corresponding IDT balances and time of vouch event, [{Voucher, Balance, Time}]
top_vouchers(User, Visited) ->
    % Contains everyone who vouched for User
    VouchersWithTime = ets:match(vouches, {{User, '$1'}, '$2'}),
    {VouchersWithBalance, _Visited} = lists:mapfoldl(
        fun([UserV, TimeV], VisitedV) ->
            {Balance, VisitedUserV} = idt_visited(UserV, VisitedV),
            {{UserV, Balance, TimeV}, VisitedUserV}
        end,
        Visited,
        VouchersWithTime),
    VouchersWithBalanceSorted = lists:reverse(lists:sort(fun({_, BalanceA, _}, {_, BalanceB, _}) -> BalanceA =< BalanceB end, VouchersWithBalance)),
    lists:sublist(VouchersWithBalanceSorted, ?TOP_VOUCHERS_SIZE).

user_idt_from_vouchers(Vouchers) ->
    Idt = lists:foldl(fun({_UserV, Balance, _TimeV}, TotalBalance) -> TotalBalance + Balance * ?IDT_REDUCE_BY_LEVEL_COEFFICIENT end, 0, Vouchers),
    trunc(math:floor(Idt)).

user_idt_from_proof(User) ->
    ProvedBalance = ets:match(id_proofs, {User, '$1', '$2', '_'}),
    case ProvedBalance of
        [] -> 0;
        [[Balance, _Time]] -> Balance;
        % unreachable
        _Rest -> 0
    end.

user_penalty_from_vouchees(Vouchees) ->
    IdtPenalty = lists:foldl(
        fun({_UserV, Balance, _TimeV}, TotalBalance) ->
            VoucheePenalty = case Balance > ?MAX_VOUCHEE_PENALTY of
                true -> ?MAX_VOUCHEE_PENALTY;
                _ -> Balance
            end,
            TotalBalance + VoucheePenalty * ?PENALTY_REDUCE_BY_LEVEL_COEFFICIENT
        end,
        0,
        Vouchees),
    trunc(math:floor(IdtPenalty)).

user_penalty_from_proof(User) ->
    ProvedPenalties = ets:match(penalties, {'_', User, '_', '$1', '$2'}),
    % use fold instead of sum to optionally add penalty age dependency
    lists:foldl(
        fun([Balance, _TimeV], TotalBalance) -> TotalBalance + Balance end,
        0,
        ProvedPenalties).

penalty_visited(User, Visited) ->
    case sets:is_element(User, Visited) of
        true -> {0, Visited};
        _ ->
            VisitedUser = sets:add_element(User, Visited),
            % Contains everyone who are vouched by the User
            VoucheesWithTime = ets:match(vouches, {{'$1', User}, '$2'}),
            {VoucheesWithBalance, _Visited} = lists:mapfoldl(
                fun([UserV, TimeV], VisitedV) ->
                    {Balance, VisitedUserV} = penalty_visited(UserV, VisitedV),
                    {{UserV, Balance, TimeV}, VisitedUserV}
                end,
                VisitedUser,
                VoucheesWithTime),
            PenaltyByVouchers = user_penalty_from_vouchees(VoucheesWithBalance),
            PenaltyByProof = user_penalty_from_proof(User),
            {PenaltyByVouchers + PenaltyByProof, VisitedUser}
    end.
