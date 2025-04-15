-module(admins).

-export([is_admin/1, admins_list/0, add_admin/2, remove_admin/2, add_admin_from_config/1]).

is_admin(User) ->
    ets:member(admins, User).

admins_list() ->
    Admins = ets:match(admins, {'$1'}),
    lists:map(fun([A]) -> A end, Admins).

add_admin(Admin, User) ->
    case admins:is_admin(Admin) of
        true -> ets:insert(admins, {User});
        _ -> {error, not_allowed}
    end.

remove_admin(Admin, User) ->
    case admins:is_admin(Admin) of
        true -> ets:delete_object(admins, {User});
        _ -> {error, not_allowed}
    end.

add_admin_from_config(User) ->
    ets:insert(admins, {User}).
