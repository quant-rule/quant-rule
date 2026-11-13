# Quant Rule — Architecture Characteristics

Rating: 1 = very low requirement, 5 = very high requirement.

## 1. Overall cost

\begin{longtable}{|L{3.0cm}|C{1.3cm}|L{1.3cm}|L{3.0cm}|L{4.6cm}|L{2.2cm}|}
\hline
\textbf{Characteristic} & \textbf{Level} & \textbf{Kata} & \textbf{Why it matters} & \textbf{Target} & \textbf{Evidence} \\
\hline
\endfirsthead
\hline
\textbf{Characteristic} & \textbf{Level} & \textbf{Kata} & \textbf{Why it matters} & \textbf{Target} & \textbf{Evidence} \\
\hline
\endhead
Overall cost & 2/5 & R1-R6 & the distributed split must stay cheap to build and run & at most 9 containers, 0 managed services, 1 host & deployment inventory \\
\hline
\end{longtable}

## 2. Structural architecture characteristics

\begin{longtable}{|L{3.0cm}|C{1.3cm}|L{1.3cm}|L{3.0cm}|L{4.6cm}|L{2.2cm}|}
\hline
\textbf{Characteristic} & \textbf{Level} & \textbf{Kata} & \textbf{Why it matters} & \textbf{Target} & \textbf{Evidence} \\
\hline
\endfirsthead
\hline
\textbf{Characteristic} & \textbf{Level} & \textbf{Kata} & \textbf{Why it matters} & \textbf{Target} & \textbf{Evidence} \\
\hline
\endhead
Simplicity & 5/5 & R1-R6 & hidden complexity hides errors & at most 5 configuration files; no infrastructure & file count \\
\hline
Modularity & 4/5 & R1, R2, R4 & data, signals, hypothesis, evaluation change independently & 0 cyclic dependencies & dependency check \\
\hline
Maintainability & 4/5 & R1-R4 & one correction must not touch unrelated parts & at most 1 module and 2 files per fix & change size \\
\hline
Testability & 5/5 & R4, R5, R6 & results must be checked against known answers & at least 90\% coverage of statistics; suite at most 5 minutes & coverage and suite time \\
\hline
Configurability & 4/5 & R1, R2, R3 & range, signals, costs, thresholds are set per hypothesis & 0 code changes to change a hypothesis & configuration diff \\
\hline
\end{longtable}

\newpage

## 3. Engineering architecture characteristics

\begin{longtable}{|L{3.0cm}|C{1.3cm}|L{1.3cm}|L{3.0cm}|L{4.6cm}|L{2.2cm}|}
\hline
\textbf{Characteristic} & \textbf{Level} & \textbf{Kata} & \textbf{Why it matters} & \textbf{Target} & \textbf{Evidence} \\
\hline
\endfirsthead
\hline
\textbf{Characteristic} & \textbf{Level} & \textbf{Kata} & \textbf{Why it matters} & \textbf{Target} & \textbf{Evidence} \\
\hline
\endhead
Evolvability & 4/5 & R1, R2 & new hypotheses and signals arrive continuously & at most 1 day per new signal & change size \\
\hline
Deployability & 2/5 & R6 & a result must reproduce elsewhere & 1 command, 0 manual steps, at most 15 minutes & timed install \\
\hline
\end{longtable}

## 4. Operational architecture characteristics

\begin{longtable}{|L{3.0cm}|C{1.3cm}|L{1.3cm}|L{3.0cm}|L{4.6cm}|L{2.2cm}|}
\hline
\textbf{Characteristic} & \textbf{Level} & \textbf{Kata} & \textbf{Why it matters} & \textbf{Target} & \textbf{Evidence} \\
\hline
\endfirsthead
\hline
\textbf{Characteristic} & \textbf{Level} & \textbf{Kata} & \textbf{Why it matters} & \textbf{Target} & \textbf{Evidence} \\
\hline
\endhead
Performance & 3/5 & R2, R4 & large history, batch experiments & at most 60 s, 4 GB RAM, 80\% of one core & benchmark \\
\hline
Reliability & 4/5 & R4, R5, R6 & completed results must not be corrupted & 50 of 50 experiments at 80\% CPU for 1 hour & constrained run \\
\hline
Responsiveness & 2/5 & R2 & exploration is interactive & at most 2 s per query & timed query \\
\hline
Scalability & 2/5 & R2, R4 & one user, growing experiment count & 10 to 100 experiments, every result correct & run count \\
\hline
Elasticity & 1/5 & - & no burst expected & 1 user, at most 2 s, at most 70\% resources & peak test \\
\hline
Fault tolerance & 2/5 & R6 & an interrupted experiment restarts & 100\% of completed results valid after a kill & kill test \\
\hline
\end{longtable}
