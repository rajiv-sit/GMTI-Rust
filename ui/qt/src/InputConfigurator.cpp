#include "InputConfigurator.h"

#include <QBoxLayout>
#include <QComboBox>
#include <QDateTime>
#include <QDir>
#include <QFile>
#include <QFileDialog>
#include <QFileInfo>
#include <QJsonDocument>
#include <QJsonObject>
#include <QLabel>
#include <QLineEdit>
#include <QNetworkAccessManager>
#include <QNetworkReply>
#include <QNetworkRequest>
#include <QPlainTextEdit>
#include <QProcess>
#include <QPushButton>
#include <QRandomGenerator>
#include <QRegularExpression>
#include <QSpinBox>
#include <QDoubleSpinBox>
#include <QStringList>
#include <QUrl>

#include <QtGlobal>

namespace
{
QString scenarioPath(const QString& root)
{
    return QDir(root).filePath("simulator/configs");
}

QString readFile(const QString& path)
{
    QFile file(path);
    if (!file.open(QIODevice::ReadOnly | QIODevice::Text)) {
        return {};
    }
    return QString::fromUtf8(file.readAll());
}

template <typename T>
T parseValue(const QString& contents, const QString& name, const T& fallback = T())
{
    const QRegularExpression rx(QStringLiteral("^%1:\\s*(\\d+)").arg(name), QRegularExpression::MultilineOption);
    const auto match = rx.match(contents);
    if (match.hasMatch()) {
        return static_cast<T>(match.captured(1).toInt());
    }
    return fallback;
}

double parseFloatValue(const QString& contents, const QString& name, double fallback)
{
    const QRegularExpression rx(QStringLiteral("^%1:\\s*([+-]?\\d+(?:\\.\\d+)?)").arg(name),
                                 QRegularExpression::MultilineOption);
    const auto match = rx.match(contents);
    if (match.hasMatch()) {
        return match.captured(1).toDouble();
    }
    return fallback;
}

quint64 parseSeedValue(const QString& contents, const QString& name, quint64 fallback)
{
    const QRegularExpression rx(QStringLiteral("^%1:\\s*(\\d+)").arg(name), QRegularExpression::MultilineOption);
    const auto match = rx.match(contents);
    if (match.hasMatch()) {
        return static_cast<quint64>(match.captured(1).toULongLong());
    }
    return fallback;
}

QString parseStringValue(const QString& contents, const QString& name)
{
    const QRegularExpression rx(QStringLiteral("^%1:\\s*(.+)").arg(name), QRegularExpression::MultilineOption);
    const auto match = rx.match(contents);
    if (match.hasMatch()) {
        return match.captured(1).trimmed();
    }
    return QString();
}
} // namespace

InputConfigurator::InputConfigurator(QWidget* parent)
    : QGroupBox(tr("Offline Test Control"), parent)
    , root_path_edit_(new QLineEdit(this))
    , browse_button_(new QPushButton(tr("Browse"), this))
    , start_button_(new QPushButton(tr("Start Engine"), this))
    , stop_button_(new QPushButton(tr("Stop Engine"), this))
    , scenario_combo_(new QComboBox(this))
    , run_button_(new QPushButton(tr("Run Scenario"), this))
    , taps_spin_(new QSpinBox(this))
    , range_spin_(new QSpinBox(this))
    , doppler_spin_(new QSpinBox(this))
    , frequency_spin_(new QDoubleSpinBox(this))
    , noise_spin_(new QDoubleSpinBox(this))
    , log_output_(new QPlainTextEdit(this))
    , server_process_(new QProcess(this))
    , network_manager_(new QNetworkAccessManager(this))
    , scenario_description_label_(new QLabel(tr("Select a scenario to load its metadata."), this))
    , scenario_seed_(0)
{
    root_path_edit_->setText(QDir::currentPath());
    root_path_edit_->setPlaceholderText(tr("Path to GMTI-Rust root"));

    auto* rootLayout = new QHBoxLayout();
    rootLayout->addWidget(new QLabel(tr("Project root:"), this));
    rootLayout->addWidget(root_path_edit_, 1);
    rootLayout->addWidget(browse_button_);

    auto* engineLayout = new QHBoxLayout();
    engineLayout->addWidget(start_button_);
    engineLayout->addWidget(stop_button_);

    auto* scenarioLayout = new QHBoxLayout();
    scenarioLayout->addWidget(new QLabel(tr("Scenario"), this));
    scenarioLayout->addWidget(scenario_combo_);
    scenarioLayout->addWidget(run_button_);

    taps_spin_->setRange(1, 32);
    taps_spin_->setValue(4);
    range_spin_->setRange(64, 8192);
    range_spin_->setSingleStep(64);
    range_spin_->setValue(2048);
    doppler_spin_->setRange(32, 1024);
    doppler_spin_->setValue(256);
    frequency_spin_->setDecimals(2);
    frequency_spin_->setRange(1.0, 200.0);
    frequency_spin_->setValue(32.0);
    noise_spin_->setDecimals(3);
    noise_spin_->setRange(0.0, 0.5);
    noise_spin_->setSingleStep(0.01);
    noise_spin_->setValue(0.03);

    scenario_description_label_->setWordWrap(true);
    scenario_description_label_->setStyleSheet("color: #cccccc;");
    scenario_description_label_->setAlignment(Qt::AlignCenter);

    auto* grid = new QGridLayout();
    grid->addWidget(new QLabel(tr("Taps"), this), 0, 0);
    grid->addWidget(taps_spin_, 0, 1);
    grid->addWidget(new QLabel(tr("Range bins"), this), 0, 2);
    grid->addWidget(range_spin_, 0, 3);
    grid->addWidget(new QLabel(tr("Doppler bins"), this), 1, 0);
    grid->addWidget(doppler_spin_, 1, 1);
    grid->addWidget(new QLabel(tr("Sine freq."), this), 1, 2);
    grid->addWidget(frequency_spin_, 1, 3);
    grid->addWidget(new QLabel(tr("Noise level"), this), 2, 0);
    grid->addWidget(noise_spin_, 2, 1);
    grid->setColumnStretch(1, 1);
    grid->setColumnStretch(3, 1);

    log_output_->setReadOnly(true);
    log_output_->setMinimumHeight(120);

    auto* layout = new QVBoxLayout(this);
    layout->addLayout(rootLayout);
    layout->addLayout(engineLayout);
    layout->addLayout(scenarioLayout);
    layout->addWidget(scenario_description_label_);
    layout->addLayout(grid);
    layout->addWidget(log_output_);

    connect(browse_button_, &QPushButton::clicked, this, &InputConfigurator::onBrowseRoot);
    connect(start_button_, &QPushButton::clicked, this, &InputConfigurator::onStartServer);
    connect(stop_button_, &QPushButton::clicked, this, &InputConfigurator::onStopServer);
    connect(run_button_, &QPushButton::clicked, this, &InputConfigurator::onRunScenario);
    connect(server_process_, &QProcess::readyReadStandardOutput, this, &InputConfigurator::onServerOutput);
    connect(server_process_, &QProcess::readyReadStandardError, this, &InputConfigurator::onServerOutput);
    connect(server_process_, QOverload<QProcess::ProcessError>::of(&QProcess::errorOccurred),
            this, &InputConfigurator::onServerError);
    connect(server_process_, QOverload<int, QProcess::ExitStatus>::of(&QProcess::finished), this, [this](int code, QProcess::ExitStatus) {
        Q_UNUSED(code);
        Q_UNUSED(QProcess::ExitStatus);
        logMessage(tr("Simulator exited."));
        updateControls();
    });

    populateScenarioList();
    connect(scenario_combo_, QOverload<int>::of(&QComboBox::currentIndexChanged), this, [this](int index) {
        const QString path = scenario_combo_->itemData(index).toString();
        if (!path.isEmpty()) {
            loadScenario(path);
            logMessage(tr("Loaded scenario %1").arg(QFileInfo(path).fileName()));
        }
    });
    if (scenario_combo_->count() > 0) {
        scenario_combo_->setCurrentIndex(0);
    }
    updateControls();
}

InputConfigurator::~InputConfigurator()
{
    if (server_process_->state() != QProcess::NotRunning) {
        server_process_->terminate();
        server_process_->waitForFinished(1000);
    }
}

void InputConfigurator::onBrowseRoot()
{
    const QString selected = QFileDialog::getExistingDirectory(this, tr("Select GMTI Workspace"), root_path_edit_->text());
    if (!selected.isEmpty()) {
        root_path_edit_->setText(selected);
        populateScenarioList();
        updateControls();
    }
}

void InputConfigurator::onStartServer()
{
    if (server_process_->state() != QProcess::NotRunning) {
        logMessage(tr("Server already running."));
        return;
    }

    const QString root = root_path_edit_->text();
    if (root.isEmpty()) {
        logMessage(tr("Set the project root before starting the engine."));
        return;
    }

    const QString cargo = QStringLiteral("cargo");
    const QStringList args = {QStringLiteral("run"),
                              QStringLiteral("--bin"),
                              QStringLiteral("simulator"),
                              QStringLiteral("--"),
                              QStringLiteral("--serve")};
    server_process_->setWorkingDirectory(root);
    server_process_->setProcessChannelMode(QProcess::MergedChannels);
    server_process_->start(cargo, args);
    if (!server_process_->waitForStarted(3000)) {
        logMessage(tr("Failed to start simulator server. Is Rust/Cargo installed?"));
        return;
    }

    logMessage(tr("Simulator server starting..."));
    updateControls();
}

void InputConfigurator::onStopServer()
{
    if (server_process_->state() == QProcess::NotRunning) {
        logMessage(tr("Server already stopped."));
        return;
    }
    server_process_->terminate();
    if (!server_process_->waitForFinished(2000)) {
        server_process_->kill();
    }
    logMessage(tr("Simulator server stopped."));
    updateControls();
}

void InputConfigurator::onRunScenario()
{
    if (server_process_->state() == QProcess::NotRunning) {
        logMessage(tr("Start the simulator engine before running scenarios."));
        return;
    }

    const int taps = taps_spin_->value();
    const int range_bins = range_spin_->value();
    const int doppler_bins = doppler_spin_->value();
    const double frequency = frequency_spin_->value();
    const double noise = noise_spin_->value();
    quint64 seed = scenario_seed_;
    if (seed == 0) {
        seed = QRandomGenerator::global()->generate64();
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("taps"), taps);
    payload.insert(QStringLiteral("range_bins"), range_bins);
    payload.insert(QStringLiteral("doppler_bins"), doppler_bins);
    payload.insert(QStringLiteral("frequency"), frequency);
    payload.insert(QStringLiteral("noise"), noise);
    payload.insert(QStringLiteral("seed"), static_cast<qint64>(seed));

    if (!current_scenario_path_.isEmpty()) {
        payload.insert(QStringLiteral("scenario"), QFileInfo(current_scenario_path_).baseName());
    }
    if (!current_scenario_description_.isEmpty()) {
        payload.insert(QStringLiteral("description"), current_scenario_description_);
    }

    logMessage(tr("Submitting offline configuration (taps=%1, range=%2, doppler=%3).")
               .arg(taps)
               .arg(range_bins)
               .arg(doppler_bins));

    const QUrl url(QStringLiteral("http://127.0.0.1:9000/ingest-config"));
    QNetworkRequest request(url);
    request.setHeader(QNetworkRequest::ContentTypeHeader, QStringLiteral("application/json"));
    auto* reply = network_manager_->post(request, QJsonDocument(payload).toJson());
    connect(reply, &QNetworkReply::finished, this, [this, reply]() {
        const bool ok = reply->error() == QNetworkReply::NoError;
        if (ok) {
            logMessage(tr("Scenario submitted successfully."));
        } else {
            logMessage(tr("Failed to submit scenario: %1").arg(reply->errorString()));
        }
        reply->deleteLater();
    });
}

void InputConfigurator::onServerOutput()
{
    const QByteArray output = server_process_->readAllStandardOutput();
    if (!output.isEmpty()) {
        logMessage(QString::fromUtf8(output).trimmed());
    }
}

void InputConfigurator::onServerError(QProcess::ProcessError error)
{
    Q_UNUSED(error)
    logMessage(tr("Simulator engine reported an error."));
    updateControls();
}

void InputConfigurator::logMessage(const QString& message)
{
    log_output_->appendPlainText(QStringLiteral("[%1] %2")
                                 .arg(QDateTime::currentDateTime().toString(Qt::ISODate), message));
}

void InputConfigurator::loadScenario(const QString& path)
{
    const QString contents = readFile(path);
    if (contents.isEmpty()) {
        return;
    }

    const int taps = parseValue<int>(contents, QStringLiteral("taps"), taps_spin_->value());
    const int range_bins = parseValue<int>(contents, QStringLiteral("range_bins"), range_spin_->value());
    const int doppler_bins = parseValue<int>(contents, QStringLiteral("doppler_bins"), doppler_spin_->value());
    const double frequency = parseFloatValue(contents, QStringLiteral("frequency"), frequency_spin_->value());
    const double noise = parseFloatValue(contents, QStringLiteral("noise"), noise_spin_->value());
    const quint64 seed = parseSeedValue(contents, QStringLiteral("seed"), 0);
    const QString description = parseStringValue(contents, QStringLiteral("description"));

    taps_spin_->setValue(taps);
    range_spin_->setValue(range_bins);
    doppler_spin_->setValue(doppler_bins);
    frequency_spin_->setValue(frequency);
    noise_spin_->setValue(noise);

    current_scenario_path_ = path;
    scenario_seed_ = seed;
    current_scenario_description_ = description;
    if (!description.isEmpty()) {
        scenario_description_label_->setText(description);
    } else {
        scenario_description_label_->setText(tr("Loaded %1").arg(QFileInfo(path).fileName()));
    }
}

void InputConfigurator::populateScenarioList()
{
    scenario_combo_->clear();
    const QString dirPath = scenarioPath(root_path_edit_->text());
    const QDir dir(dirPath);
    if (!dir.exists()) {
        logMessage(tr("Scenario directory not found: %1").arg(dirPath));
        return;
    }

    const auto files = dir.entryInfoList(QStringList{QStringLiteral("*.yaml")}, QDir::Files);
    for (const auto& file : files) {
        scenario_combo_->addItem(file.fileName(), file.absoluteFilePath());
    }
}

void InputConfigurator::updateControls()
{
    const bool running = server_process_->state() != QProcess::NotRunning;
    start_button_->setEnabled(!running);
    stop_button_->setEnabled(running);
    run_button_->setEnabled(running);
}
