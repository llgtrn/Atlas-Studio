-module(server).
-behaviour(gen_server).
-export([start/0, handle_request/1]).
-export([init/1, handle_call/3, handle_cast/2]).

-include("shared.hrl").

-type state() :: #{requests := [#request{}]}.

-callback on_request(#request{}) -> ok | {error, term()}.

-deprecated([{old_api, 1, "use handle_request/1 instead"}]).

-spec start() -> {ok, pid()}.
start() ->
    gen_server:start(?MODULE, [], []).

-spec init(term()) -> {ok, state()}.
init(_Args) ->
    {ok, #{requests => []}}.

-spec handle_call(term(), term(), state()) -> {reply, term(), state()}.
handle_call({request, Req}, _From, State) ->
    Result = process_request(Req),
    {reply, Result, State};
handle_call(_Request, _From, State) ->
    {reply, ok, State}.

-spec handle_cast(term(), state()) -> {noreply, state()}.
handle_cast(_Msg, State) ->
    {noreply, State}.

-spec handle_request(#request{}) -> ok | {error, term()}.
handle_request(#request{retries = R}) when R > ?MAX_RETRIES ->
    {error, max_retries};
handle_request(Req) ->
    process_request(Req).

-spec old_api(term()) -> ok.
old_api(_X) -> ok.

process_request(#request{id = Id, payload = Payload}) ->
    ?TRANSFORM({Id, Payload}).
