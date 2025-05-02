using System.Diagnostics;
using System.Threading.Tasks.Dataflow;
using SpacetimeDB.Types;

string[] space_names = ["WAN", "FOX", "ONO", "ROG", "YUK", "KES", "REI", "ASH"];
string[] time_names = ["LIZ", "PRI", "UMA", "DOC", "LEO", "TER", "RAV", "ORI"];

const int players = 4;
const double shoutProbability = 0.2;
const string moduleName = "space-vs-time";
const string serverUrl = "http://space-vs-time-server:3000";
const string authDir = "./auth";
string[] throwShouts = ["KYAAH!", "SOREEE!", "METEOR SHOT!", "GIGA STRIKE!", "EAT THIS!", "FOCUS...", "TRUE AIM!", "MY TURN!", "BYE BYE", "IN THE ZONE"];
string[] hitShouts = ["OOF", "WHY ME?", "NOT AGAIN...", "I FAILED U", "MY CAREER", "NO...", "SORRY...", "WONT GIVE UP", "GAAH", "SONNA BAKANA", "IN THE FACE", "NOT BAD", "TCH."];
bool shouldLogPlayerActivity = Environment.GetEnvironmentVariable("LOG_PLAYER_ACTIVITY") != "0";

await UntilServerAvailable();

Directory.CreateDirectory(authDir);

SimulationContext[] contexts = space_names.Take(players).Concat(time_names.Take(players))
    // .Take(1)
    .Select(name => new SimulationContext(name, new BroadcastBlock<Report>(a => a)))
    .ToArray();

Task[] tasks = contexts.Select(RunSimulation).ToArray();

using PeriodicTimer timer = new(TimeSpan.FromMilliseconds(500));
while (await timer.WaitForNextTickAsync())
{
    if (shouldLogPlayerActivity)
    {
        Console.Clear();
        Console.WriteLine("Name\tPing\tUpdates\tHashCode");
    }

    foreach (var context in contexts)
    {
        Report report = context.Reporter.Receive();
        if (shouldLogPlayerActivity)
        {
            Console.WriteLine($"{context.Name}\t{report.Ping}ms\t{report.Invocations}\t{report.HashCode}");
        }
    }
}

await Task.WhenAll(tasks);

async Task RunSimulation(SimulationContext ctx)
{
    FileInfo tokenFile = new FileInfo($"{authDir}/{ctx.Name}.txt");
    Stopwatch updateStopwatch = new();
    EntityState myLastKnownState = EntityState.Standby;
    Vector3D myLastKnownPosition = new Vector3D();
    EntityKind myKind = EntityKind.Ball;

    TimeSpan limit = TimeSpan.FromSeconds(1);
    int invocations = 0;
    int ping = 0;
    int hashcode = 0;

    string? token = tokenFile.Exists ? File.ReadAllText(tokenFile.FullName) : null;
    DbConnection? conn = null;
    conn = Connect();

    using PeriodicTimer timer = new(TimeSpan.FromMilliseconds(1000.0 / 100)); // 60fps, i.e. 16.67ms interval
    while (await timer.WaitForNextTickAsync())
    {
        conn.FrameTick();

        if (updateStopwatch.Elapsed > limit)
        {
            ctx.Reporter.Post(new Report(invocations, ping, hashcode));
            updateStopwatch.Restart();
            invocations = 0;
            conn.Reducers.Ping(Stopwatch.GetTimestamp());
        }
        //if (conn.IsActive) Update(conn);
    }

    DbConnection Connect() {
        return DbConnection.Builder()
        .WithUri(serverUrl)
        .WithModuleName(moduleName)
        .WithToken(token)
        .OnDisconnect((cb, exc) => {
            if (conn != null) {
                conn.Db.Movables.OnUpdate -= Movables_OnUpdate;
                conn.Db.Pingables.OnUpdate -= Pingables_OnUpdate;
            }

            conn = Connect();
        })
        .OnConnect((connection, identity, authToken) => { File.WriteAllText(tokenFile.FullName, authToken); Init(connection); })
        .OnConnectError(exc => {
            Console.WriteLine($"Error while connecting: {exc}");
            if (conn != null) {
                conn.Db.Movables.OnUpdate -= Movables_OnUpdate;
                conn.Db.Pingables.OnUpdate -= Pingables_OnUpdate;
            }

            conn = Connect();
        })
        .OnDisconnect((conn, exc) => Console.WriteLine("Disconnected"))
        .Build();
    }

    void Init(DbConnection conn)
    {
        updateStopwatch.Start();
        // conn.Db.Movables.OnUpdate += Movables_OnUpdate;
        conn.Db.Movables.OnUpdate += Movables_OnUpdate;
        conn.Db.Pingables.OnUpdate += Pingables_OnUpdate;
        conn.SubscriptionBuilder().Subscribe([
            $"SELECT * FROM movables",
            $"SELECT * FROM pingables WHERE name='{ctx.Name}'"]);
        conn.Reducers.Join(ctx.Name);
        Console.WriteLine($"Player {ctx.Name} joins the game");
    }

    void Movables_OnUpdate(EventContext context, Movable oldRow, Movable newRow)
    {
        if (newRow.Name == ctx.Name)
        {
            hashcode = newRow.GetHashCode();
            myLastKnownState = newRow.State;
            myLastKnownPosition = newRow.Position;
            myKind = newRow.Kind;
            if (newRow.State == EntityState.ReadyToMove)
            {
                Vector3D? position = GetBallAssignment(context, ctx.Name, newRow.Kind);
                if (position is not null)
                {
                    context.Reducers.MoveTo(position.X, position.Y);
                }
            }
            else if (newRow.State == EntityState.ReadyToThrow)
            {
                // context.Reducers.Say("KYAAH!");
                EntityKind otherKind = newRow.Kind == EntityKind.SpacePlayer ? EntityKind.TimePlayer : EntityKind.SpacePlayer;
                Movable? other = context.Db.Movables.Iter().Where(i => i.Kind == otherKind).OrderBy(i => Random.Shared.Next()).FirstOrDefault();
                if (other is not null) {
                    context.Reducers.ThrowTo(other.Position.X, other.Position.Y);
                    ShoutIfPossible(context, throwShouts);
                }
            }
            else if (newRow.State == EntityState.Hit && oldRow.State != newRow.State) {
                ShoutIfPossible(context, hitShouts);
            }
        }
        else if (newRow.Kind == EntityKind.Ball && context.Db.Movables.Iter().FirstOrDefault(i => i.Name == ctx.Name)?.State == EntityState.ReadyToMove)
        {
            Vector3D? position = GetBallAssignment(context, ctx.Name, myKind);
            if (position is not null)
            {
                context.Reducers.MoveTo(position.X, position.Y);
            }
        }

        invocations++;
    }

    Vector3D? GetBallAssignment(EventContext ctx, string name, EntityKind playerKind) {
        var balls = ctx.Db
            .Movables.Iter()
            .Where(m => (m.Kind == EntityKind.Ball) && ((playerKind == EntityKind.SpacePlayer && m.Position.X < 0) || (playerKind == EntityKind.TimePlayer && m.Position.X > 0)))
            .ToList();
        
        var players = ctx.Db
            .Movables.Iter()
            .Where(m => m.Kind == playerKind)
            .ToList();

        foreach (var ball in balls)
        {
            var closestPlayer = players.OrderBy(m => GetDistance(m.Position, ball.Position)).FirstOrDefault();
            if (closestPlayer is not null) {
                if (closestPlayer.Name == name) {
                    return ball.Position;
                } else {
                    players.Remove(closestPlayer);
                }
            }
        }

        return null;
    }

    float GetDistance(Vector3D vector1, Vector3D vector2) {
        float dx = vector1.X - vector2.X;
        float dy = vector1.Y - vector2.Y;
        float dz = vector2.Z - vector2.Z;
        return Convert.ToSingle(Math.Sqrt(dx * dx + dy * dy + dz * dz));
    }

    void Pingables_OnUpdate(EventContext context, Pingable oldRow, Pingable newRow)
    {
        if (newRow.Name != ctx.Name)
        {
            throw new InvalidOperationException("Not me");
        }

        ping = Convert.ToInt32(TimeSpan.FromTicks(Stopwatch.GetTimestamp() - newRow.Timestamp).TotalMilliseconds);
    }
}

void ShoutIfPossible(EventContext context, string[] shouts)
{
    if (Random.Shared.NextDouble() <= shoutProbability)
    {
        context.Reducers.Say(shouts[Random.Shared.Next(shouts.Length)]);
    }
}

async Task UntilServerAvailable()
{
    using HttpClient http = new();
    while (true)
    {
        try
        {
            HttpResponseMessage response = await http.GetAsync($"{serverUrl}/v1/database/{moduleName}");
            response.EnsureSuccessStatusCode();
            Console.Error.WriteLine($"Server is available, starting simulation with {players} players per team...");
            return;
        }
        catch
        {
            Console.Error.WriteLine("Server module is not yet available, retrying in 5s...");
            await Task.Delay(5000);
        }
    }
}

record Report(int Invocations, int Ping, int HashCode);
record SimulationContext(string Name, BroadcastBlock<Report> Reporter);