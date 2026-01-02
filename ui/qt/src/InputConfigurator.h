#pragma once

#include <QGroupBox>

class QComboBox;
class QDoubleSpinBox;
class QLineEdit;
class QPlainTextEdit;
class QPushButton;
class QProcess;
class QNetworkAccessManager;

class InputConfigurator : public QGroupBox
{
    Q_OBJECT

public:
    explicit InputConfigurator(QWidget* parent = nullptr);
    ~InputConfigurator() override;

private slots:
    void onBrowseRoot();
    void onStartServer();
    void onStopServer();
    void onRunScenario();
    void onServerOutput();
    void onServerError(QProcess::ProcessError error);

private:
    void logMessage(const QString& message);
    void loadScenario(const QString& path);
    void populateScenarioList();
    void updateControls();

    QLineEdit* root_path_edit_;
    QPushButton* browse_button_;
    QPushButton* start_button_;
    QPushButton* stop_button_;
    QComboBox* scenario_combo_;
    QPushButton* run_button_;
    QSpinBox* taps_spin_;
    QSpinBox* range_spin_;
    QSpinBox* doppler_spin_;
    QDoubleSpinBox* frequency_spin_;
    QDoubleSpinBox* noise_spin_;
    QPlainTextEdit* log_output_;
    QProcess* server_process_;
    QNetworkAccessManager* network_manager_;
    QLabel* scenario_description_label_;
    QString current_scenario_path_;
    QString current_scenario_description_;
    quint64 scenario_seed_;
};
